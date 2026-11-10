use super::*;

/// Exercise the complete draw/coverage path, including glass faded to zero by daylight,
/// with no shadow tiles. The made-up pane and lamp require no original OMSI content.
#[test]
#[ignore = "requires a graphics adapter; run with --ignored on a GPU host"]
fn cabin_glare_is_local_and_respects_switches_and_coverage() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let mut renderer = pollster::block_on(Renderer::new_with(
        &instance,
        None,
        Some(wgpu::TextureFormat::Rgba8UnormSrgb),
        RenderOptions {
            msaa: 1,
            ssao: false,
            shadow_size: 512,
            fxaa: false,
            render_scale: 1.0,
            light_shadows: 0,
            ..Default::default()
        },
    ))
    .expect("test renderer");
    let mut scene = renderer.new_scene();
    let mesh = renderer.add_mesh(
        &mut scene,
        &MeshData {
            positions: vec![
                Vec3::new(-1.5, 2.0, 0.0),
                Vec3::new(1.5, 2.0, 0.0),
                Vec3::new(1.5, 2.0, 3.0),
                Vec3::new(-1.5, 2.0, 3.0),
            ],
            normals: vec![-Vec3::Y; 4],
            uvs: vec![glam::Vec2::ZERO; 4],
            ranges: vec![(0, 6, 0)],
            indices: vec![0, 1, 2, 0, 2, 3],
            one_sided: false,
        },
    );
    let pane = renderer.add_texture(
        &mut scene,
        &omsi_texture::Image {
            width: 1,
            height: 1,
            rgba: vec![0, 0, 0, 64],
            has_alpha: true,
        },
        false,
    );
    let hole = renderer.add_texture(
        &mut scene,
        &omsi_texture::Image {
            width: 1,
            height: 1,
            rgba: vec![0, 0, 0, 0],
            has_alpha: true,
        },
        false,
    );
    let extra = MaterialExtra {
        glass: true,
        no_z_write: true,
        daylight_glass: true,
        ..Default::default()
    };
    let glass = renderer.add_material_extra(
        &mut scene,
        Some(pane),
        AlphaMode::Blend,
        [1.0; 4],
        false,
        None,
        None,
        None,
        None,
        [0.0; 3],
        extra,
    );
    let clear = renderer.add_material_extra(
        &mut scene,
        Some(hole),
        AlphaMode::Blend,
        [1.0; 4],
        false,
        None,
        None,
        None,
        None,
        [0.0; 3],
        extra,
    );
    let hidden = renderer.add_material_extra(
        &mut scene,
        Some(pane),
        AlphaMode::Blend,
        [1.0; 4],
        false,
        None,
        None,
        None,
        None,
        [0.0; 3],
        MaterialExtra {
            daylight_glass: false,
            ..extra
        },
    );
    let id = renderer.add_instance(&mut scene, mesh, DVec3::ZERO, Mat4::IDENTITY, vec![glass]);
    scene.instances[id].slot_alpha[0] = 0.0;
    let camera = Camera {
        position: DVec3::new(0.0, 0.0, 1.5),
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
        fov_deg: 70.0,
        near: 0.1,
        far: 100.0,
    };
    let night = Lighting {
        enhanced: true,
        shadows: false,
        fog_density: 0.0,
        night: 1.0,
        sun_dir: Vec3::new(0.0, 0.0, -1.0),
        inside: Some((DVec3::ZERO, 0.0, [3.0, 4.0, 3.0, 0.0, 0.0, 1.5])),
        ..Default::default()
    };
    scene.interior_lights.push(PointLight {
        position: camera.position,
        color: [1.0, 0.8, 0.5],
        intensity: 0.0,
        radius: 2.0,
        ..Default::default()
    });
    let draw = |r: &mut Renderer, s: &mut Scene, l: &Lighting| {
        r.render_to_image(s, 128, 128, &camera, l).unwrap()
    };
    let off = draw(&mut renderer, &mut scene, &night);
    scene.interior_lights[0].intensity = 1.0;
    let on = draw(&mut renderer, &mut scene, &night);
    let sum = |image: &[u8], x: usize, y: usize| {
        image[(y * 128 + x) * 4..][..3]
            .iter()
            .map(|&x| x as i32)
            .sum::<i32>()
    };
    let gain = sum(&on, 64, 64) - sum(&off, 64, 64);
    assert!(
        gain > 40,
        "the lit fixture must reflect on a daylight-faded pane: gain {gain}"
    );
    assert!(
        (sum(&on, 32, 64) - sum(&off, 32, 64)).abs() <= 6,
        "glare must not wash across the pane"
    );
    let copy = scene.interior_lights[0];
    scene.interior_lights.push(copy);
    let duplicate = draw(&mut renderer, &mut scene, &night);
    assert!(
        (sum(&duplicate, 64, 64) - sum(&on, 64, 64)).abs() <= 3,
        "duplicate lamp sets must not multiply glare"
    );
    renderer.set_material(&mut scene, id, 0, clear);
    let through_hole = draw(&mut renderer, &mut scene, &night);
    assert!(
        (sum(&through_hole, 64, 64) - sum(&off, 64, 64)).abs() <= 3,
        "transparent texture holes must stay clear"
    );
    renderer.set_material(&mut scene, id, 0, hidden);
    let faded = draw(&mut renderer, &mut scene, &night);
    assert!(
        (sum(&faded, 64, 64) - sum(&off, 64, 64)).abs() <= 3,
        "other script fades must still hide the layer"
    );
    renderer.set_material(&mut scene, id, 0, glass);
    let outside = draw(
        &mut renderer,
        &mut scene,
        &Lighting {
            inside: None,
            ..night.clone()
        },
    );
    assert!(
        (sum(&outside, 64, 64) - sum(&off, 64, 64)).abs() <= 3,
        "a camera outside the bus must get no cabin glare"
    );
    let day = Lighting {
        night: 0.0,
        sun_dir: Vec3::Z,
        ..night.clone()
    };
    let day_on = draw(&mut renderer, &mut scene, &day);
    for lamp in &mut scene.interior_lights {
        lamp.intensity = 0.0;
    }
    let day_off = draw(&mut renderer, &mut scene, &day);
    assert!(
        sum(&day_on, 64, 64) - sum(&day_off, 64, 64) < gain / 2,
        "daylight must subdue the reflection"
    );
}
