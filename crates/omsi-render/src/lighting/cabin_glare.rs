//! A small, view-local list of cabin lamps for reflections in the player's glass.

use crate::*;

/// The closest eight distinct lit fixtures, independent of the lamp-shadow setting.
/// Interior-light sets repeat the same fixture for several meshes: reflecting each copy
/// would multiply its brightness. Keep the brightest copy and use the existing GPU list.
pub(crate) fn cabin_glare_lamps(
    lamps: &[PointLight],
    eye: DVec3,
    vehicle: Option<&(DVec3, f64, [f32; 6])>,
) -> [[f32; 4]; 2] {
    let mut nearest = [(usize::MAX, f64::INFINITY); 8];
    let Some(vehicle) = vehicle.filter(|v| point_in_vehicle_box(eye, v)) else {
        return [[-1.0; 4]; 2];
    };
    let bounds = passes::VehicleBox::new(vehicle, vehicle.0);
    for (i, lamp) in lamps.iter().enumerate() {
        let strength = lamp.intensity * lamp.color.iter().sum::<f32>();
        if !lamp.position.is_finite()
            || !strength.is_finite()
            || strength <= 0.001
            || !bounds.contains((lamp.position - vehicle.0).as_vec3(), 0.1)
        {
            continue;
        }
        let distance = lamp.position.distance_squared(eye);
        // The shader fades fixtures out by six metres from the pane. Eight metres
        // from the eye leaves room for the glass in front without scanning the rear deck.
        if distance > 64.0 {
            continue;
        }
        if let Some((at, _)) = nearest.iter_mut().find(|(at, _)| {
            *at != usize::MAX && lamps[*at].position.distance_squared(lamp.position) < 0.0025
        }) {
            let previous = &lamps[*at];
            if strength > previous.intensity * previous.color.iter().sum::<f32>() {
                *at = i;
            }
            continue;
        }
        if let Some(at) = nearest.iter().position(|(_, d)| distance < *d) {
            nearest.copy_within(at..7, at + 1);
            nearest[at] = (i, distance);
        }
    }
    std::array::from_fn(|r| {
        std::array::from_fn(|c| {
            let at = nearest[r * 4 + c].0;
            if at == usize::MAX { -1.0 } else { at as f32 }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lamp(position: DVec3, intensity: f32) -> PointLight {
        PointLight {
            position,
            intensity,
            color: [1.0; 3],
            ..Default::default()
        }
    }

    #[test]
    fn only_lit_local_fixtures_are_reflected_once() {
        let origin = DVec3::new(1_000_000.0, -2_000_000.0, 20.0);
        let bus = (origin, 90.0, [2.5, 12.0, 3.0, 0.0, 0.0, 1.5]);
        let eye = origin + DVec3::new(4.0, 0.0, 1.7);
        let front = origin + DVec3::new(3.0, 0.0, 2.5);
        let lamps = [
            lamp(front, 0.4),
            lamp(front, 1.0),
            lamp(front + DVec3::Y * 4.0, 1.0), // the neighbouring bus
            lamp(front - DVec3::X, 0.0),
            lamp(front - DVec3::X * 2.0, f32::NAN),
            lamp(DVec3::NAN, 1.0),
            lamp(front - DVec3::X * 2.0, 0.8),
        ];
        let selected = cabin_glare_lamps(&lamps, eye, Some(&bus));
        assert_eq!(selected, [[1.0, 6.0, -1.0, -1.0], [-1.0; 4]]);
        assert_eq!(cabin_glare_lamps(&lamps, eye, None), [[-1.0; 4]; 2]);
        assert_eq!(
            cabin_glare_lamps(&lamps, eye + DVec3::Y * 4.0, Some(&bus)),
            [[-1.0; 4]; 2]
        );
        let off: Vec<_> = lamps.iter().map(|l| lamp(l.position, 0.0)).collect();
        assert_eq!(cabin_glare_lamps(&off, eye, Some(&bus)), [[-1.0; 4]; 2]);
    }

    #[test]
    fn selection_is_bounded_and_keeps_the_nearest_fixtures() {
        let bus = (DVec3::ZERO, 0.0, [2.5, 20.0, 3.0, 0.0, 0.0, 1.5]);
        let eye = DVec3::new(0.0, 8.0, 1.7);
        let lamps: Vec<_> = (0..16)
            .map(|i| lamp(DVec3::new(0.0, i as f64 - 7.0, 2.5), 1.0))
            .collect();
        assert_eq!(
            cabin_glare_lamps(&lamps, eye, Some(&bus)),
            [[15.0, 14.0, 13.0, 12.0], [11.0, 10.0, 9.0, 8.0]]
        );
    }
}
