// A nearby cabin fixture seen in the inner face of a pane. OMSI's interior lights
// give positions and colours, not luminous geometry: approximate a small diffuser
// and its faint halo, never a uniform wash across the glass. The reflected ray puts
// it in the same place in the world as the driver's head moves.
fn cabin_glare(p: vec3<f32>, n: vec3<f32>, v: vec3<f32>) -> vec3<f32> {
    let r = reflect(-v, n);
    let nv = abs(dot(n, v));
    let fresnel = min(0.04 + 0.96 * pow(1.0 - nv, 5.0), 0.3);
    // A fixture is much brighter than a lit window. Pre-exposure makes its reflection
    // unobtrusive by day and visible in the dark; cap it before Fresnel so a dark sky
    // cannot turn the small halo into a blinding patch.
    let radiance = min(enh.exposure.z * 40.0, 6.0);
    var glare = vec3<f32>(0.0);
    for (var i = 0u; i < 8u; i = i + 1u) {
        let at = camera.glare_lamps[i / 4u][i % 4u];
        if (at < 0.0) { break; }
        let lamp = lights[u32(at)];
        let d = lamp.pos.xyz - p;
        let along = dot(d, r);
        // Both eye and fixture must be on the same side of the pane. A fixture
        // beyond the glass is transmitted, not reflected back into the cabin.
        if (along <= 0.0 || dot(d, n) * dot(v, n) <= 0.0) { continue; }
        let reach = 1.0 - smoothstep(3.0, 6.0, length(d));
        let miss = d - r * along;
        let q2 = dot(miss, miss) / (0.12 * 0.12);
        let image = exp(-2.0 * q2) + 0.035 * exp(-0.18 * q2);
        glare = glare + max(lamp.color.rgb, vec3<f32>(0.0)) * clamp(lamp.color.w, 0.0, 1.0)
            * (image * reach * radiance * fresnel);
    }
    return glare;
}
