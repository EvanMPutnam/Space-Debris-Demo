//! Mapping between the TEME frame SGP4 works in and Bevy's world.
//!
//! The world is inertial: one unit is one Earth radius, +Y points to the north
//! pole, and the Earth spins under the orbits. TEME (x toward the equinox,
//! z north) maps to world (x, z, -y), a proper rotation.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;
use sgp4_rust::JulianDate;

pub const EARTH_RADIUS_KM: f64 = 6378.135;

pub fn teme_to_world(teme_km: [f64; 3]) -> Vec3 {
    let [x, y, z] = teme_km.map(|c| (c / EARTH_RADIUS_KM) as f32);
    Vec3::new(x, z, -y)
}

/// Rotation of the Earth mesh at `time`.
///
/// Bevy's UV sphere has its poles on ±Z and puts longitude λ at angle λ + π
/// around Z. Turning it by π about Z lines Greenwich up with +X (Earth-fixed);
/// GMST then turns Earth-fixed into TEME, and the last step maps TEME to world.
pub fn earth_rotation(time: JulianDate) -> Quat {
    let gmst = time.gmst() as f32;
    Quat::from_rotation_x(-FRAC_PI_2) * Quat::from_rotation_z(gmst + PI)
}

/// Unit vector toward the Sun in world coordinates, from the Astronomical
/// Almanac's low-precision formula (good to about 0.01 degree).
pub fn sun_direction(time: JulianDate) -> Vec3 {
    let n = (time.day - 2_451_545.0) + time.fraction;
    let mean_longitude = (280.460 + 0.985_647_4 * n).to_radians();
    let mean_anomaly = (357.528 + 0.985_600_3 * n).to_radians();
    let ecliptic_longitude = mean_longitude
        + (1.915 * mean_anomaly.sin() + 0.020 * (2.0 * mean_anomaly).sin()).to_radians();
    let obliquity = (23.439 - 0.000_000_4 * n).to_radians();
    let (sin_l, cos_l) = ecliptic_longitude.sin_cos();
    let teme = [cos_l, obliquity.cos() * sin_l, obliquity.sin() * sin_l];
    teme_to_world(teme.map(|c| c * EARTH_RADIUS_KM)).normalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greenwich_faces_the_gmst_direction() {
        let time = JulianDate::from_calendar(2026, 3, 20, 12, 0, 0.0);
        // Longitude 0 on the mesh: angle π around Z.
        let greenwich_on_mesh = Vec3::new(-1.0, 0.0, 0.0);
        let world = earth_rotation(time) * greenwich_on_mesh;
        let gmst = time.gmst();
        let expected = teme_to_world([
            gmst.cos() * EARTH_RADIUS_KM,
            gmst.sin() * EARTH_RADIUS_KM,
            0.0,
        ]);
        assert!(world.distance(expected) < 1e-5, "{world} vs {expected}");
        // North pole of the mesh points up.
        assert!((earth_rotation(time) * Vec3::Z).distance(Vec3::Y) < 1e-6);
    }

    #[test]
    fn sun_is_overhead_near_noon_at_greenwich_on_the_equinox() {
        let time = JulianDate::from_calendar(2026, 3, 20, 12, 0, 0.0);
        let greenwich = earth_rotation(time) * Vec3::new(-1.0, 0.0, 0.0);
        let angle = greenwich.angle_between(sun_direction(time)).to_degrees();
        assert!(angle < 3.0, "{angle}");
    }
}
