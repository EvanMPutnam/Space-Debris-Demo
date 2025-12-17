use bevy::prelude::*;
use chrono::{Datelike, Timelike, Utc};

use crate::loader::load_tles_to_sat_rec;
use SGP4_Rust::ext::jday;
use SGP4_Rust::propagation::SatRec;

pub const EARTH_RADIUS_KM: f64 = 6378.137;
pub const KM_TO_WORLD: f32 = (1.0 / EARTH_RADIUS_KM) as f32;

#[derive(Component)]
pub struct Debris {
    pub sat_index: usize,
}
pub struct DebrisSat {
    pub satrec: SatRec,
}

#[derive(Resource)]
pub struct DebrisField {
    pub sats: Vec<DebrisSat>,
}

#[derive(Resource)]
pub struct SimulationTime {
    /// Integer part of JD at app start.
    pub base_jd: f64,
    /// Fractional part of JD at app start.
    pub base_fr: f64,
    /// How fast sim time runs vs real time (1.0 = real time).
    pub time_scale: f64,
}

pub fn setup_simulation_time(mut commands: Commands) {
    let now = Utc::now();

    let year = now.year() as i32;
    let month = now.month() as i32;
    let day = now.day() as i32;

    let hour = now.hour() as i32;
    let minute = now.minute() as i32;
    let second = now.second() as i32;
    let sec_f = second as f64 + now.nanosecond() as f64 * 1e-9;

    // Your `jday` returns a single f64: full Julian date in days.
    let jd_full = jday(year, month, day, hour, minute, sec_f);

    let base_jd = jd_full.floor();
    let base_fr = jd_full - base_jd;

    commands.insert_resource(SimulationTime {
        base_jd,
        base_fr,
        time_scale: 1.0, // 1× real time
    });
}

pub fn setup_debris_field(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // TODO - Update to asset loader.
    let sats = load_tles_to_sat_rec("assets/tle_sample.txt")
        .iter()
        .map(|sat| DebrisSat {
            satrec: sat.clone(),
        })
        .collect::<Vec<_>>();
    let sat_length = sats.len();

    commands.insert_resource(DebrisField { sats });

    // Shared mesh / material for debris points.
    let debris_mesh = meshes.add(Sphere::new(0.03).mesh().uv(8, 4));
    let debris_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.2, 0.2),
        unlit: true,
        ..default()
    });

    // Spawn an entity per satellite.
    for i in 0..sat_length {
        commands.spawn((
            Name::new(format!("Debris {}", i)),
            Debris { sat_index: i },
            Mesh3d(debris_mesh.clone()),
            MeshMaterial3d(debris_material.clone()),
            Transform::default(),
            GlobalTransform::default(),
        ));
    }
}

pub fn update_debris_positions(
    time: Res<Time>,
    sim_time: Res<SimulationTime>,
    mut debris_field: ResMut<DebrisField>,
    mut query: Query<(&Debris, &mut Transform)>,
) {
    // Real seconds since app start
    let elapsed_secs = time.elapsed().as_secs_f64();

    // Convert to days and apply time scale (1.0 = real time)
    let delta_days = (elapsed_secs / 86_400.0) * sim_time.time_scale;

    // Full JD (integer + fractional) at this frame
    let jd_full = (sim_time.base_jd + sim_time.base_fr) + delta_days;
    let jd = jd_full.floor();
    let fr = jd_full - jd;

    for (debris, mut transform) in &mut query {
        if let Some(debris_sat) = debris_field.sats.get_mut(debris.sat_index) {
            let satrec = &mut debris_sat.satrec;

            let (_err, r_km, _v_km_s) = match satrec.sgp4(jd, fr) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Error parsing TLE: {}", e);
                    continue;
                }
            };

            let x = r_km[0] as f32;
            let y = r_km[2] as f32;
            let z = r_km[1] as f32;

            let pos = Vec3::new(x, y, z) * KM_TO_WORLD;
            transform.translation = pos;
        }
    }
}
