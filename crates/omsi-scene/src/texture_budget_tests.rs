use super::*;

struct Fixture(PathBuf);

impl Fixture {
    fn world() -> (Self, World) {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "openomsi-texture-budget-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join("global.cfg"),
            "[name]\nTexture budget regression\n",
        )
        .unwrap();
        let world = World::open(&path, &path.join("global.cfg"), 20261101).unwrap();
        (Self(path), world)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let (Ok(path), Ok(root)) = (self.0.canonicalize(), std::env::temp_dir().canonicalize()) {
            if path.starts_with(&root) && path != root {
                let _ = std::fs::remove_dir_all(path);
            }
        }
    }
}

fn renderer() -> Renderer {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::NOOP;
    descriptor.backend_options.noop = wgpu::NoopBackendOptions { enable: true };
    let instance = wgpu::Instance::new(descriptor);
    pollster::block_on(Renderer::new_with(
        &instance,
        None,
        Some(wgpu::TextureFormat::Rgba8UnormSrgb),
        omsi_render::RenderOptions {
            msaa: 1,
            ssao: false,
            shadow_size: 1024,
            ..Default::default()
        },
    ))
    .unwrap()
}

fn data(side: u32) -> TextureData {
    TextureData::from_image(Image {
        width: side,
        height: side,
        rgba: [200, 40, 20, 255].repeat((side * side) as usize),
        has_alpha: false,
    })
}

fn scenery(
    world: &World,
    renderer: &Renderer,
    scene: &mut Scene,
    name: &str,
    tile: (i32, i32),
) -> (PathBuf, TextureId) {
    let path = world.root.join(name);
    let picture = data(1024);
    let id = renderer.add_texture_data(scene, &picture);
    world
        .gpu
        .lock()
        .textures
        .insert(path.clone(), TexEntry::uploaded(id, &picture, scene));
    world
        .tile_state
        .lock()
        .entry(tile)
        .or_default()
        .gpu
        .shared_textures
        .push(path.clone());
    (path, id)
}

#[test]
fn texture_budget_shrinks_far_scenery_before_near_scenery() {
    let (_fixture, world) = Fixture::world();
    let renderer = renderer();
    let mut scene = renderer.new_scene();
    let (_, near) = scenery(&world, &renderer, &mut scene, "near.png", (0, 0));
    let (_, far) = scenery(&world, &renderer, &mut scene, "far.png", (4, 0));
    let centers = [DVec3::new(150.0, 150.0, 0.0)];
    world.set_texture_budget(8_000_000);
    world.update_texture_budget(&renderer, &mut scene, &centers, true);
    assert_eq!(scene.texture_size_of(near), Some((1024, 1024)));
    assert_eq!(scene.texture_size_of(far), Some((512, 512)));
    assert!(world.gpu.lock().wants_restore.is_empty());
    // Far textures alone cannot meet this budget. Near scenery must remain textured.
    world.set_texture_budget(500_000);
    world.update_texture_budget(&renderer, &mut scene, &centers, true);
    assert_eq!(scene.texture_size_of(near), Some((256, 256)));
    assert!(scene.texture_bytes_of(far) > 0);
    assert!(renderer.texture_bytes(&scene) <= 500_000);
    // Standing still must not immediately undo the reductions.
    world.update_texture_budget(&renderer, &mut scene, &centers, true);
    assert!(world.gpu.lock().wants_restore.is_empty());
}

#[test]
fn texture_budget_reserves_pending_restores_and_rechecks_finished_work() {
    let (_fixture, world) = Fixture::world();
    let renderer = renderer();
    let mut scene = renderer.new_scene();
    let (a, first) = scenery(&world, &renderer, &mut scene, "a.png", (0, 0));
    let (b, second) = scenery(&world, &renderer, &mut scene, "b.png", (0, 0));
    let original = Arc::new(data(1024));
    for (path, id) in [(&a, first), (&b, second)] {
        assert!(renderer.drop_top_levels(&mut scene, id, 2));
        world
            .gpu
            .lock()
            .textures
            .get_mut(path)
            .unwrap()
            .record_upload(&original, &scene);
    }
    let centers = [DVec3::new(150.0, 150.0, 0.0)];
    world.set_texture_budget(8_000_000);
    world.update_texture_budget(&renderer, &mut scene, &centers, true);
    let queued = std::mem::take(&mut world.gpu.lock().wants_restore);
    assert_eq!(queued.len(), 1, "only one full image fits");
    world.upgrades_pending.lock().insert(queued[0].clone());
    world.update_texture_budget(&renderer, &mut scene, &centers, true);
    assert!(
        world.gpu.lock().wants_restore.is_empty(),
        "the same room cannot be spent twice"
    );
    let id = world.gpu.lock().textures[&queued[0]].id;
    // Memory was used elsewhere while the worker decoded the full image.
    world.set_texture_budget(700_000);
    world
        .upgrades_done
        .lock()
        .push((queued[0].clone(), original.clone(), None));
    world.apply_texture_upgrades(&renderer, &mut scene, None);
    assert_eq!(scene.texture_size_of(id), Some((256, 256)));
    assert_eq!(world.gpu.lock().textures[&queued[0]].dropped, 2);
    world.set_texture_budget(100_000_000);
    world
        .upgrades_done
        .lock()
        .push((queued[0].clone(), original, None));
    world.apply_texture_upgrades(&renderer, &mut scene, None);
    assert_eq!(scene.texture_size_of(id), Some((1024, 1024)));
    assert_eq!(world.gpu.lock().textures[&queued[0]].dropped, 0);
}

#[test]
fn texture_budget_recovers_a_placeholder_without_room_for_the_full_source() {
    let (_fixture, world) = Fixture::world();
    let renderer = renderer();
    let mut scene = renderer.new_scene();
    let (path, id) = scenery(&world, &renderer, &mut scene, "lost.png", (0, 0));
    let original = Arc::new(data(1024));
    renderer.free_texture(&mut scene, id);
    world
        .gpu
        .lock()
        .textures
        .get_mut(&path)
        .unwrap()
        .record_upload(&original, &scene);
    world.set_texture_budget(1_000_000);
    world.update_texture_budget(
        &renderer,
        &mut scene,
        &[DVec3::new(150.0, 150.0, 0.0)],
        true,
    );
    assert_eq!(
        std::mem::take(&mut world.gpu.lock().wants_restore),
        vec![path.clone()]
    );
    world
        .upgrades_done
        .lock()
        .push((path.clone(), original, None));
    world.apply_texture_upgrades(&renderer, &mut scene, None);
    assert!(scene.texture_bytes_of(id) > 0);
    assert_eq!(scene.texture_size_of(id), Some((256, 256)));
    assert_eq!(world.gpu.lock().textures[&path].dropped, 2);
}

#[test]
fn texture_restore_headroom_accounts_for_the_rest_of_the_card() {
    let card = omsi_render::GpuMemory {
        textures: 100,
        meshes: 200,
        other: 600,
        budget: Some(1000),
        measured: true,
    };
    assert_eq!(texture_restore_room(1000, 0, card), 0);
    assert_eq!(texture_restore_room(1000, 700, card), 0);
    let free = omsi_render::GpuMemory {
        textures: 100,
        ..Default::default()
    };
    assert_eq!(texture_restore_room(1000, 0, free), 750);
    assert_eq!(budget_level_drop(4096, 13, 256, 1000, 999), 4);
    assert_eq!(budget_level_drop(256, 9, 256, 1000, 999), 0);
}

#[test]
fn a_rectangular_fleet_texture_can_give_way_without_losing_its_picture() {
    let (_fixture, world) = Fixture::world();
    let renderer = renderer();
    let mut scene = renderer.new_scene();
    let picture = TextureData::from_image(Image {
        width: 2048,
        height: 128,
        rgba: [200, 40, 20, 255].repeat(2048 * 128),
        has_alpha: false,
    });
    let id = renderer.add_texture_data(&mut scene, &picture);
    world
        .vehicle_textures
        .lock()
        .insert(world.root.join("fleet.png"), (id, 1));
    let centers = [DVec3::ZERO];
    world.set_texture_budget(50_000);
    world.update_texture_budget(&renderer, &mut scene, &centers, true);
    assert_eq!(scene.texture_size_of(id), Some((256, 16)));
    assert!(scene.texture_bytes_of(id) > 0 && scene.texture_bytes_of(id) <= 50_000);
    // An impossible budget must keep the final useful level, rather than a blank slot.
    world.set_texture_budget(1);
    world.update_texture_budget(&renderer, &mut scene, &centers, true);
    assert_eq!(scene.texture_size_of(id), Some((256, 16)));
    assert!(scene.texture_bytes_of(id) > 0);
}

#[test]
fn a_failed_restore_waits_then_retries_even_when_the_camera_is_still() {
    let (_fixture, world) = Fixture::world();
    let renderer = renderer();
    let mut scene = renderer.new_scene();
    let (path, id) = scenery(&world, &renderer, &mut scene, "retry.png", (0, 0));
    assert!(renderer.drop_top_levels(&mut scene, id, 2));
    {
        let mut gpu = world.gpu.lock();
        let entry = gpu.textures.get_mut(&path).unwrap();
        entry.record_upload(&data(1024), &scene);
        entry.retry_after = Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
    }
    world.set_texture_budget(100_000_000);
    let centers = [DVec3::new(150.0, 150.0, 0.0)];
    assert_eq!(
        world.update_texture_budget(&renderer, &mut scene, &centers, true),
        0
    );
    assert!(world.gpu.lock().wants_restore.is_empty());
    world
        .gpu
        .lock()
        .textures
        .get_mut(&path)
        .unwrap()
        .retry_after = Some(std::time::Instant::now());
    world.budget_checked.lock().checked = None;
    assert_eq!(
        world.update_texture_budget(&renderer, &mut scene, &centers, false),
        1
    );
    assert_eq!(world.gpu.lock().wants_restore, vec![path]);
}
