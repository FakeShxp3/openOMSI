use super::*;

/// Test the actual glass shader, including a pane without illumination_interior.
#[test]
#[ignore = "requires a graphics adapter; run with --ignored on a GPU host"]
fn interior_glare_tracks_switches_and_only_affects_enhanced_cabin_glass() {
    exercise_glare(false, 1.0, 0.1);
}

#[test]
#[ignore = "requires a graphics adapter; run with --ignored on a GPU host"]
fn interior_glare_survives_daylight_fade_but_not_other_fades_or_empty_masks() {
    exercise_glare(true, 0.0, 0.1);
    exercise_glare(false, 0.0, 0.1);
    exercise_glare(true, 0.0, 0.0);
}

fn exercise_glare(daylight_glass: bool, fade: f32, authored_alpha: f32) {
    let adapter = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let mut renderer = pollster::block_on(Renderer::new_with(
        &adapter, None, Some(wgpu::TextureFormat::Rgba8UnormSrgb),
        RenderOptions { msaa: 1, ssao: false, shadow_size: 1024, fxaa: false, render_scale: 1.0, ..Default::default() },
    )).expect("test renderer");
    let mut scene = renderer.new_scene();
    let pane = renderer.add_material_extra(
        &mut scene, None, AlphaMode::Blend, [0.0, 0.0, 0.0, authored_alpha], false,
        None, None, None, None, [0.0; 3],
        MaterialExtra { daylight_glass, glass: true, no_z_write: true, ..Default::default() },
    );
    let mesh = renderer.add_mesh(&mut scene, &MeshData {
        positions: vec![Vec3::new(-1.0, 2.0, -1.0), Vec3::new(1.0, 2.0, -1.0), Vec3::new(1.0, 2.0, 1.0), Vec3::new(-1.0, 2.0, 1.0)],
        normals: vec![-Vec3::Y; 4], uvs: vec![glam::Vec2::ZERO; 4],
        ranges: vec![(0, 6, 0)], indices: vec![0, 1, 2, 0, 2, 3], one_sided: false,
    });
    let instance = renderer.add_instance(&mut scene, mesh, DVec3::ZERO, Mat4::IDENTITY, vec![pane]);
    scene.instances[instance].slot_alpha[0] = fade;
    let first = renderer.alloc_interior_lights(&mut scene, 1);
    renderer.set_reflected_lamps(&mut scene, instance, first, 1);
    assert_eq!(scene.instances[instance].interior_lamps, 0, "reflection must not assign diffuse lights");
    let camera = Camera { position: DVec3::ZERO, yaw: 0.0, pitch: 0.0, roll: 0.0, fov_deg: 60.0, near: 0.1, far: 100.0 };
    let mut lighting = Lighting {
        enhanced: true, shadows: false, fog_density: 0.0, night: 1.0,
        sun_intensity: 0.0, ambient: Vec3::splat(0.001),
        inside: Some((DVec3::ZERO, 0.0, [2.0, 4.0, 2.0, 0.0, 0.0, 0.0])),
        ..Default::default()
    };
    let mut render = |lighting: &Lighting, position: DVec3, intensity: f32| {
        renderer.set_interior_light(&mut scene, first, PointLight {
            position, radius: 100.0, core: 2.0, color: [1.0, 0.8, 0.5], intensity, ..Default::default()
        });
        renderer.render_to_image(&mut scene, 64, 64, &camera, lighting).unwrap()
    };
    let off = render(&lighting, DVec3::ZERO, 0.0);
    let on = render(&lighting, DVec3::ZERO, 1.0);
    if authored_alpha == 0.0 || (fade == 0.0 && !daylight_glass) {
        assert_eq!(off, on, "ordinary fades and empty authored masks must stay invisible");
        return;
    }
    // A ceiling lamp's highlight is above the centre, not at the eye-level test pixel.
    let overhead = render(&lighting, DVec3::new(0.0, 0.0, 1.0), 1.0);
    assert!(overhead.chunks_exact(4).zip(off.chunks_exact(4))
        .any(|(a, b)| a[0] > b[0].saturating_add(10)), "overhead lamp must reflect");
    let pixel = (32 * 64 + 32) * 4;
    assert!(on[pixel] > off[pixel].saturating_add(10), "a nearby lit lamp should reflect");
    let moved = render(&lighting, DVec3::new(0.8, 0.0, 0.0), 1.0);
    assert!(moved[pixel] < on[pixel], "the reflection must follow the lamp's position");
    // The light must disappear on switching off, behind the pane, and beyond its reach.
    assert_eq!(off, render(&lighting, DVec3::ZERO, 0.0));
    assert_eq!(off, render(&lighting, DVec3::new(0.0, 3.0, 0.0), 1.0));
    assert_eq!(off, render(&lighting, DVec3::new(0.0, -5.0, 0.0), 1.0));
    for (enhanced, night, inside) in [(false, 1.0, lighting.inside), (true, 0.0, lighting.inside), (true, 1.0, None)] {
        lighting.enhanced = enhanced;
        lighting.night = night;
        lighting.inside = inside;
        assert_eq!(render(&lighting, DVec3::ZERO, 0.0), render(&lighting, DVec3::ZERO, 1.0),
            "no glare outside Enhanced night cabin view");
    }
}
