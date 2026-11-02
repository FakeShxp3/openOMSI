use super::*;
use crate::paint::{Color, Painter};
use glam::Vec3;

fn quad(p: &mut Painter, lo: Vec2, hi: Vec2, color: Color) {
    p.world_poly(
        &[
            lo.extend(0.0),
            Vec3::new(hi.x, lo.y, 0.0),
            hi.extend(0.0),
            Vec3::new(lo.x, hi.y, 0.0),
        ],
        color,
    );
}

fn covers(verts: &[Vertex], point: Vec2, color: Color) -> bool {
    verts.chunks_exact(3).any(|t| {
        if t[0]
            .color
            .iter()
            .zip(color.0)
            .any(|(a, b)| (*a - b).abs() > 1e-5)
        {
            return false;
        }
        let p: [Vec2; 3] = std::array::from_fn(|i| Vec2::new(t[i].pos[0], t[i].pos[1]));
        let signs = [0, 1, 2].map(|i| (p[(i + 1) % 3] - p[i]).perp_dot(point - p[i]));
        signs.iter().all(|s| *s >= -1e-5) || signs.iter().all(|s| *s <= 1e-5)
    })
}

#[test]
fn a_ground_that_fits_is_byte_for_byte_the_original_geometry() {
    let (mut exact, mut bounded) = (Painter::new(), Painter::with_ground_budget(120, 10.0, 0.25));
    for p in [&mut exact, &mut bounded] {
        quad(p, Vec2::splat(-8.0), Vec2::splat(8.0), Color::hex(0x304020));
        p.ribbon(
            &[Vec3::new(-7.0, -2.0, 0.0), Vec3::new(6.0, 2.0, 0.0)],
            1.0,
            2.0,
            Color::WHITE,
            true,
        );
    }
    assert!(!bounded.ground_reduced());
    assert_eq!(bounded.finish_ground(), exact.verts);
}

#[test]
fn a_full_land_layer_keeps_late_roads_at_both_ends_of_the_map() {
    let green = Color::hex(0x408020);
    let mut p = Painter::with_ground_budget(96, 8.0, 0.1);
    // The old first-come budget filled here and discarded every later road.
    for _ in 0..16 {
        quad(&mut p, Vec2::splat(-8.0), Vec2::splat(8.0), green);
    }
    for x in [-6.0, 6.0] {
        p.ribbon(
            &[Vec3::new(x, -7.0, 0.0), Vec3::new(x, 7.0, 0.0)],
            0.2,
            0.0,
            Color::WHITE,
            false,
        );
    }
    assert!(p.ground_reduced());
    assert!(
        !p.at_vertex_limit(),
        "a coarsened map must still accept later layers"
    );
    let verts = p.finish_ground();
    assert!(verts.len() <= 96);
    for x in [-6.0, 6.0] {
        assert!(covers(&verts, Vec2::new(x, 0.0), Color::WHITE));
    }
    assert!(
        covers(&verts, Vec2::ZERO, green),
        "land between the roads remains"
    );
}

#[test]
fn even_a_checkerboard_never_exceeds_the_vertex_budget() {
    for limit in [6, 17, 96, 260_000] {
        let mut grid = Grid::new(limit, 500.0, 1.0);
        for y in 0..grid.side {
            for x in 0..grid.side {
                grid.cells[y * grid.side + x] = if (x + y) % 2 == 0 {
                    [255; 4]
                } else {
                    [0, 0, 0, 255]
                };
            }
        }
        let verts = grid.vertices();
        assert!(verts.len() <= limit, "{} > {limit}", verts.len());
        assert_eq!(verts.len() % 6, 0);
        assert!(
            verts
                .iter()
                .all(|v| v.pos.iter().all(|q| q.is_finite() && q.abs() <= 500.0))
        );
    }
}

#[test]
fn winding_clipping_and_nonfinite_input_do_not_drop_a_late_layer() {
    let mut p = Painter::with_ground_budget(24, 10.0, 1.0);
    for _ in 0..5 {
        quad(&mut p, Vec2::splat(-1e6), Vec2::splat(1e6), Color::BLACK);
    }
    p.world_poly(&[Vec3::splat(f32::NAN), Vec3::X, Vec3::Y], Color::WHITE);
    p.world_poly(
        &[
            Vec3::new(-10.0, -10.0, 0.0),
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(10.0, -10.0, 0.0),
        ],
        Color::WHITE,
    );
    let verts = p.finish_ground();
    assert!(verts.len() <= 24);
    assert!(
        verts
            .chunks_exact(3)
            .any(|t| t[0].color[0] > 0.5 && covers(t, Vec2::ZERO, Color(t[0].color)))
    );
    assert!(verts.iter().all(|v| v.pos.iter().all(|x| x.is_finite())));
}

#[test]
fn clearing_a_reduced_painter_starts_an_exact_ground_again() {
    let mut p = Painter::with_ground_budget(6, 10.0, 1.0);
    for _ in 0..2 {
        quad(&mut p, Vec2::ZERO, Vec2::ONE, Color::WHITE);
    }
    assert!(p.ground_reduced());
    p.clear();
    quad(&mut p, Vec2::ZERO, Vec2::ONE, Color::BLACK);
    assert!(!p.ground_reduced());
    assert_eq!(p.finish_ground().len(), 6);
}

#[test]
fn dense_ground_keeps_a_median_between_nearby_roads() {
    let p = nearby_roads();
    assert!(p.ground_reduced());
    let verts = p.finish_ground();
    assert!(verts.len() <= 12_000);
    for x in [-2.5, 2.5] {
        assert!(covers(&verts, Vec2::new(x, 0.4), Color::WHITE));
    }
    for x in [-5.0, 0.4, 5.0] {
        assert!(
            !covers(&verts, Vec2::new(x, 0.4), Color::WHITE),
            "road expanded into x={x}"
        );
        assert!(covers(&verts, Vec2::new(x, 0.4), Color::hex(0x408020)));
    }
}

fn nearby_roads() -> Painter {
    let green = Color::hex(0x408020);
    let mut p = Painter::with_ground_budget(12_000, 640.0, 0.4);
    quad(&mut p, Vec2::splat(-640.0), Vec2::splat(640.0), green);
    for _ in 0..4_000 {
        p.world_poly(
            &[
                Vec3::new(600.0, 600.0, 0.0),
                Vec3::new(600.01, 600.0, 0.0),
                Vec3::new(600.0, 600.01, 0.0),
            ],
            green,
        );
    }
    for (left, right) in [(-4.0, -1.0), (1.0, 4.0)] {
        quad(
            &mut p,
            Vec2::new(left, -20.0),
            Vec2::new(right, 20.0),
            Color::WHITE,
        );
    }
    p
}

#[test]
fn a_uniform_fine_grid_needs_only_one_quad() {
    let mut grid = Grid::new(6, 640.0, 0.01);
    assert!(grid.cells.len() * std::mem::size_of::<[u8; 4]>() <= 64 * 1024 * 1024);
    grid.cells.fill([64, 128, 32, 255]);
    let verts = grid.vertices();
    assert_eq!(verts.len(), 6);
    for point in [Vec2::ZERO, Vec2::splat(-639.0), Vec2::splat(639.0)] {
        assert!(covers(&verts, point, Color::hex(0x408020)));
    }
}

#[test]
fn coarsening_filters_alpha_in_linear_premultiplied_space() {
    let mut grid = Grid::new(6, 1.0, 1.0);
    grid.cells = vec![[255, 0, 0, 255], [0, 0, 255, 255], [0; 4], [0; 4]];
    let verts = grid.vertices();
    assert_eq!(verts.len(), 6);
    let c = verts[0].color;
    assert!((c[0] - srgb(0.5)).abs() < 1.0 / 255.0);
    assert_eq!(c[1], 0.0);
    assert!((c[2] - srgb(0.5)).abs() < 1.0 / 255.0);
    assert!((c[3] - 0.5).abs() < 1.0 / 255.0);
}

#[test]
fn merging_rectangles_preserves_every_cell_and_transparent_hole() {
    let mut grid = Grid::new(1000, 4.0, 1.0);
    let colors = [[0; 4], [255; 4], [32, 64, 128, 255], [128, 64, 32, 128]];
    for y in 0..8 {
        for x in 0..8 {
            grid.cells[y * 8 + x] = colors[if x < 3 { 1 } else { [0, 2, 3][(x + y / 2) % 3] }];
        }
    }
    let expected = grid.cells.clone();
    let verts = grid.vertices();
    assert_eq!(
        verts.iter().filter(|v| v.color == Color::WHITE.0).count(),
        6
    );
    for y in 0..8 {
        for x in 0..8 {
            let point = Vec2::new(x as f32 - 3.7, y as f32 - 3.6);
            let want = expected[y * 8 + x].map(|c| c as f32 / 255.0);
            let hits: Vec<_> = verts
                .chunks_exact(3)
                .filter(|t| covers(t, point, Color(t[0].color)))
                .collect();
            assert_eq!(hits.is_empty(), want[3] == 0.0, "cell {x},{y}");
            for t in hits {
                assert_eq!(t[0].color, want, "cell {x},{y}");
            }
        }
    }
}

#[test]
#[ignore = "requires a graphics adapter; checks the navigation fallback at 4x MSAA"]
fn reduced_roads_keep_their_median_in_the_gpu_picture() {
    use crate::gpu::{Draw, Gpu, Layer};
    let verts = nearby_roads().finish_ground();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut gpu = Gpu::new(&device, format, 4, 256);
    let size = 256;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("navigation ground test"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (size * size * 4) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.upload(&device, &queue, 0, &verts);
    let mut layer = Layer::flat([0.0, 0.0, size as f32, size as f32], 0.0, 1.0);
    layer.view_proj = glam::Mat4::orthographic_rh(-8.0, 8.0, -8.0, 8.0, -1.0, 1.0);
    layer.viewport = [0.0, 0.0, size as f32, size as f32];
    let mut encoder = device.create_command_encoder(&Default::default());
    gpu.render(
        &device,
        &queue,
        &mut encoder,
        &target.create_view(&Default::default()),
        (size, size),
        Some(wgpu::Color::TRANSPARENT),
        &[layer],
        &[Draw {
            buffer: 0,
            range: 0..verts.len() as u32,
            layer: 0,
            texture: 0,
        }],
    );
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size * 4),
                rows_per_image: Some(size),
            },
        },
        target.size(),
    );
    let (tx, rx) = std::sync::mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |r| {
        tx.send(r).unwrap();
    });
    queue.submit([encoder.finish()]);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let pixels = readback.slice(..).get_mapped_range();
    for (x, road) in [
        (48, false),
        (88, true),
        (128, false),
        (168, true),
        (208, false),
    ] {
        let pixel = &pixels[((128 * size + x) * 4) as usize..][..4];
        if road {
            assert!(pixel[..3].iter().all(|v| *v > 240), "road: {pixel:?}");
        } else {
            assert!(
                pixel[1] > pixel[0] + 30 && pixel[1] > pixel[2] + 50,
                "median or verge: {pixel:?}"
            );
        }
    }
    println!(
        "{} vertices; roads and median remain distinct at 4x MSAA",
        verts.len()
    );
}
