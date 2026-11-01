//! Admission and resolution choices shared by texture streaming and recovery.
use super::*;

pub(super) fn fit_upgrade(data: &Arc<TextureData>, allowed: u64) -> Option<Arc<TextureData>> {
    let mut upload = data.clone();
    while upload.gpu_bytes() > allowed {
        let side = upload.width.max(upload.height);
        if side <= 1 {
            return None;
        }
        let mut small = omsi_render::fit_texture(&upload, side / 2)?;
        // A compressed texture with non-block-aligned sides is decoded by the renderer;
        // admission must count that RGBA allocation, not the smaller BC source.
        if small.format.is_compressed() && (small.width % 4 != 0 || small.height % 4 != 0) {
            let rgba =
                small
                    .format
                    .decode_level(small.levels.first()?, small.width, small.height, true);
            small.format = omsi_texture::PixelFormat::Rgba8;
            small.levels = vec![rgba];
            small.gpu_mips = true;
        }
        upload = Arc::new(small);
    }
    Some(upload)
}

/// Leave room for streaming and for the old and new allocations to coexist briefly.
pub(super) fn texture_restore_room(
    limit: u64,
    video_limit: u64,
    card: omsi_render::GpuMemory,
) -> u64 {
    let mut room = if limit == 0 {
        u64::MAX
    } else {
        (limit - limit / 100 * 15).saturating_sub(card.textures)
    };
    let cap = card.cap(video_limit);
    if cap > 0 {
        room = room.min((cap - cap / 10).saturating_sub(card.total()));
    }
    room
}

pub(super) fn texture_restore_cost(e: &TexEntry) -> u64 {
    if e.bytes == 0 {
        // Recover a missing picture at a modest size even if the full source cannot fit.
        e.full_bytes.min(128 * 128 * 4 * 4 / 3)
    } else {
        e.full_bytes.saturating_sub(e.bytes)
    }
}

pub(super) fn budget_level_drop(side: u32, levels: u32, floor: u32, bytes: u64, over: u64) -> u32 {
    let mut n = 0;
    while n + 1 < levels && side.checked_shr(n + 1).unwrap_or(0) >= floor {
        n += 1;
        if bytes.saturating_sub(bytes >> (2 * n).min(63)) >= over {
            break;
        }
    }
    n
}
#[cfg(test)]
mod budget_memo_tests {
    use super::*;

    #[test]
    fn an_idle_look_is_kept_until_something_changes() {
        let mut m = BudgetMemo::default();
        let c = [DVec3::new(10.0, 20.0, 5.0)];
        let limit = 1000;
        assert!(!m.unchanged(1, 900, limit, &c), "nothing remembered yet");
        m.remember(1, 900, limit, &c, true);
        assert!(m.unchanged(1, 900, limit, &c));
        // a little way on, or up a hill: the same distances for the budget's purposes
        assert!(m.unchanged(1, 900, limit, &[DVec3::new(30.0, 20.0, 50.0)]));
        assert!(
            !m.unchanged(1, 900, limit, &[DVec3::new(40.0, 20.0, 5.0)]),
            "moved 30 m"
        );
        assert!(!m.unchanged(2, 900, limit, &c), "the tiles changed");
        assert!(!m.unchanged(1, 900, limit, &[c[0], c[0]]), "another center");
        // a texture compressed meanwhile: a little less or more, still nothing to do
        assert!(m.unchanged(1, 895, limit, &c));
        assert!(m.unchanged(1, 990, limit, &c));
        assert!(!m.unchanged(1, 1001, limit, &c), "over the budget");
        assert!(
            !m.unchanged(1, 900, 600, &c),
            "a lower budget must run even while standing still"
        );
        assert!(
            !m.unchanged(1, 880, limit, &c),
            "room for a texture to come back"
        );
        m.remember(1, 900, limit, &c, false);
        assert!(
            !m.unchanged(1, 900, limit, &c),
            "a look that did something looks again"
        );
    }
}

impl World {
    /// OMSI's `[texmemlimit]`: the scenery and vehicle textures may take `bytes` on the GPU
    /// (0 = no limit), see [`World::update_texture_budget`].
    pub fn set_texture_budget(&self, bytes: u64) {
        self.texture_limit
            .store(bytes, std::sync::atomic::Ordering::Relaxed);
    }

    /// What the game may hold on the graphics card all together (bytes; 0 = no more than
    /// the driver's budget): over it, the textures give way as over their own budget (see
    /// [`World::update_texture_budget`]), since they are what can.
    pub fn set_video_memory_limit(&self, bytes: u64) {
        self.video_memory_limit
            .store(bytes, std::sync::atomic::Ordering::Relaxed);
    }

    /// The textures' budget now (bytes, 0 = none).
    pub fn texture_budget_bytes(&self) -> u64 {
        self.texture_limit
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}
