//! An orbit camera around the Earth with smoothed motion and a slow drift
//! when left alone.

use std::f32::consts::FRAC_PI_2;

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    post_process::bloom::Bloom,
    prelude::*,
};

use crate::astro::sun_direction;
use crate::clock::SimClock;

const MIN_RADIUS: f32 = 1.3;
const MAX_RADIUS: f32 = 60.0;
const MAX_PITCH: f32 = FRAC_PI_2 - 0.02;
const ROTATE_SPEED: f32 = 0.0045;
const ZOOM_SPEED: f32 = 0.12;
/// How quickly the view catches up with its target, per second.
const SMOOTHING: f32 = 9.0;
/// Seconds without input before the camera starts to drift.
const IDLE_SECONDS: f32 = 6.0;
const DRIFT_SPEED: f32 = 0.025;

/// Where the camera is heading (`target_*`) and where it is now.
#[derive(Component)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub radius: f32,
    target_yaw: f32,
    target_pitch: f32,
    target_radius: f32,
    idle: f32,
    /// Pixels the pointer moved while the button was held, to tell a click from a drag.
    pub drag_distance: f32,
}

impl OrbitCamera {
    pub fn new(yaw: f32, pitch: f32, radius: f32) -> Self {
        Self {
            yaw,
            pitch,
            radius,
            target_yaw: yaw,
            target_pitch: pitch,
            target_radius: radius,
            idle: 0.0,
            drag_distance: 0.0,
        }
    }

    fn transform(&self) -> Transform {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let direction = Vec3::new(cos_yaw * cos_pitch, sin_pitch, sin_yaw * cos_pitch);
        Transform::from_translation(direction * self.radius).looking_at(Vec3::ZERO, Vec3::Y)
    }
}

/// Starts looking at the Earth from a little past the Sun's direction, so the
/// view shows the day side, the terminator, and the first city lights.
pub fn spawn_camera(mut commands: Commands, clock: Res<SimClock>) {
    let sun = sun_direction(clock.now);
    let orbit = OrbitCamera::new(sun.z.atan2(sun.x) + 1.15, 0.32, 4.6);
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 0.72,
            near: 0.01,
            far: 2000.0,
            ..default()
        }),
        Tonemapping::TonyMcMapface,
        Bloom {
            intensity: 0.12,
            ..Bloom::NATURAL
        },
        orbit.transform(),
        orbit,
    ));
}

pub fn control_camera(
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    camera: Single<(&mut Transform, &mut OrbitCamera)>,
) {
    let (mut transform, mut orbit) = camera.into_inner();
    let dt = time.delta_secs();
    let mut active = false;

    if buttons.just_pressed(MouseButton::Left) {
        orbit.drag_distance = 0.0;
    }
    if buttons.pressed(MouseButton::Left) && motion.delta != Vec2::ZERO {
        orbit.drag_distance += motion.delta.length();
        // Rotate less when zoomed in, so the surface tracks the pointer.
        let speed = ROTATE_SPEED * (orbit.target_radius / 4.0).clamp(0.25, 1.5);
        orbit.target_yaw += motion.delta.x * speed;
        orbit.target_pitch =
            (orbit.target_pitch + motion.delta.y * speed).clamp(-MAX_PITCH, MAX_PITCH);
        active = true;
    }

    let lines = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
    };
    if lines != 0.0 {
        orbit.target_radius = (orbit.target_radius * (1.0 - lines * ZOOM_SPEED).max(0.2))
            .clamp(MIN_RADIUS, MAX_RADIUS);
        active = true;
    }

    orbit.idle = if active { 0.0 } else { orbit.idle + dt };
    if orbit.idle > IDLE_SECONDS {
        let ramp = ((orbit.idle - IDLE_SECONDS) / 3.0).min(1.0);
        orbit.target_yaw += DRIFT_SPEED * ramp * dt;
    }

    let blend = 1.0 - (-SMOOTHING * dt).exp();
    orbit.yaw += (orbit.target_yaw - orbit.yaw) * blend;
    orbit.pitch += (orbit.target_pitch - orbit.pitch) * blend;
    // Zoom in log space so it feels even at every distance.
    orbit.radius =
        (orbit.radius.ln() + (orbit.target_radius.ln() - orbit.radius.ln()) * blend).exp();

    *transform = orbit.transform();
}
