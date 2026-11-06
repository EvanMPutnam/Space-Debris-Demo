//! Click an object to select it; the selection shows its orbit, ground track,
//! and a line down to the point beneath it.

use std::f32::consts::PI;

use bevy::{prelude::*, window::PrimaryWindow};
use sgp4_rust::{Geodetic, JulianDate, frames::teme_to_ecef};

use crate::astro::{earth_rotation, teme_to_world};
use crate::camera::OrbitCamera;
use crate::catalog::Catalog;
use crate::clock::SimClock;
use crate::objects::{Motion, Tracked};

/// Selected at startup if present.
const DEFAULT_CATALOG_NUMBER: u32 = 25544; // ISS (ZARYA)
const PICK_RADIUS_PX: f32 = 14.0;
const CLICK_SLOP_PX: f32 = 5.0;
const PATH_SAMPLES: usize = 360;

#[derive(Resource, Default)]
pub struct Selection {
    pub index: Option<usize>,
}

pub struct SelectionPlugin;

impl Plugin for SelectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Selection>()
            .add_systems(Startup, select_default)
            .add_systems(
                Update,
                (
                    pick,
                    draw_selection.after(crate::objects::propagate_objects),
                ),
            );
    }
}

fn select_default(catalog: Res<Catalog>, mut selection: ResMut<Selection>) {
    selection.index = catalog
        .entries
        .iter()
        .position(|e| e.tle.catalog_number == DEFAULT_CATALOG_NUMBER);
}

fn pick(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform, &OrbitCamera)>,
    objects: Query<(&Tracked, &Motion, &ViewVisibility)>,
    mut selection: ResMut<Selection>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        selection.index = None;
    }
    let (camera, camera_transform, orbit) = *camera;
    if !buttons.just_released(MouseButton::Left) || orbit.drag_distance > CLICK_SLOP_PX {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let eye = camera_transform.translation();

    let nearest = objects
        .iter()
        .filter(|(_, motion, visible)| {
            visible.get() && motion.valid && !behind_earth(eye, motion.position)
        })
        .filter_map(|(tracked, motion, _)| {
            let screen = camera
                .world_to_viewport(camera_transform, motion.position)
                .ok()?;
            Some((tracked.index, screen.distance(cursor)))
        })
        .filter(|(_, distance)| *distance < PICK_RADIUS_PX)
        .min_by(|a, b| a.1.total_cmp(&b.1));

    if let Some((index, _)) = nearest {
        selection.index = Some(index);
    }
}

/// Whether the Earth blocks the line of sight from `eye` to `point`.
fn behind_earth(eye: Vec3, point: Vec3) -> bool {
    let to_point = point - eye;
    let length = to_point.length();
    let direction = to_point / length;
    let along = (-eye).dot(direction);
    if along <= 0.0 || along >= length {
        return false;
    }
    (eye + direction * along).length() < 1.0
}

fn draw_selection(
    selection: Res<Selection>,
    catalog: Res<Catalog>,
    clock: Res<SimClock>,
    camera: Single<&GlobalTransform, With<Camera3d>>,
    mut gizmos: Gizmos,
) {
    let Some(index) = selection.index else { return };
    let satellite = &catalog.entries[index].satellite;
    let now = clock.now;
    let Ok(state) = satellite.propagate_to(now) else {
        return;
    };
    let position = teme_to_world(state.position);
    let period = satellite.period_minutes();

    // One revolution ahead, fading out as it wraps around.
    let path = (0..=PATH_SAMPLES).filter_map(|i| {
        let fraction = i as f64 / PATH_SAMPLES as f64;
        let state = satellite
            .propagate_to(now.add_minutes(fraction * period))
            .ok()?;
        let fade = 1.0 - fraction as f32;
        Some((
            teme_to_world(state.position),
            LinearRgba::rgb(1.6, 1.7, 1.9) * (0.15 + 0.85 * fade),
        ))
    });
    gizmos.linestrip_gradient(path);

    // Ground track for the same revolution, drawn on the rotating Earth.
    let earth = earth_rotation(now);
    let ground = |time: JulianDate| -> Option<Vec3> {
        let state = satellite.propagate_to(time).ok()?;
        Some(surface_point(
            earth,
            teme_to_ecef(state.position, time),
            1.003,
        ))
    };
    let track = (0..=PATH_SAMPLES).filter_map(|i| {
        let fraction = i as f64 / PATH_SAMPLES as f64;
        let point = ground(now.add_minutes(fraction * period))?;
        Some((
            point,
            LinearRgba::rgb(2.4, 1.5, 0.35) * (1.0 - fraction as f32 * 0.85),
        ))
    });
    gizmos.linestrip_gradient(track);

    // A line straight down to the geodetic point below the object.
    let below = Geodetic::from_teme(state.position, now);
    let surface = Geodetic {
        altitude_km: 0.0,
        ..below
    };
    let nadir = surface_point(earth, surface.to_ecef(), 1.0);
    gizmos.line_gradient(
        position,
        nadir,
        LinearRgba::rgb(2.0, 2.0, 2.2),
        LinearRgba::rgb(2.4, 1.5, 0.35),
    );

    // A ring facing the camera around the object.
    let eye = camera.translation();
    let size = eye.distance(position) * 0.018;
    let facing = Quat::from_rotation_arc(Vec3::Z, (eye - position).normalize());
    gizmos
        .circle(
            Isometry3d::new(position, facing),
            size,
            LinearRgba::rgb(2.5, 2.5, 2.8),
        )
        .resolution(48);
}

/// World position of an Earth-fixed point, scaled to `radius` Earth radii.
/// The Earth mesh puts Greenwich at angle π around its Z axis.
fn surface_point(earth: Quat, ecef_km: [f64; 3], radius: f32) -> Vec3 {
    let [x, y, z] = ecef_km.map(|c| c as f32);
    let mesh_local = Quat::from_rotation_z(PI) * Vec3::new(x, y, z);
    earth * mesh_local.normalize() * radius
}
