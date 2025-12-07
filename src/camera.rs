// src/camera.rs
use std::{f32::consts::FRAC_PI_2, ops::Range};

use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
};

/// Global settings for the orbit camera (speed + limits)
#[derive(Debug, Resource)]
pub struct CameraSettings {
    /// Min / max distance from target (Earth center)
    pub radius_range: Range<f32>,
    /// Allowed pitch range so we never flip over the poles
    pub pitch_range: Range<f32>,
    /// How fast mouse movement rotates the camera
    pub rotate_speed: f32,
    /// How fast scroll zooms in/out
    pub zoom_speed: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        let pitch_limit = FRAC_PI_2 - 0.01; // just shy of ±90°
        Self {
            radius_range: 1.5..50.0,
            pitch_range: -pitch_limit..pitch_limit,
            rotate_speed: 0.005,
            zoom_speed: 0.15,
        }
    }
}

/// State for an orbit camera:
/// yaw   -> rotation around Y (longitude)
/// pitch -> tilt north/south (latitude)
/// radius -> distance from target
#[derive(Component)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub radius: f32,
    pub target: Vec3,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            // Start on the equator, looking from +X toward the origin
            yaw: 0.0,
            pitch: 0.0,
            radius: 4.0,
            target: Vec3::ZERO,
        }
    }
}

impl OrbitCamera {
    /// Convert yaw/pitch/radius into a Transform
    fn update_transform(&self, transform: &mut Transform) {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();

        // Direction from target to camera
        let dir = Vec3::new(
            cos_yaw * cos_pitch, // X
            sin_pitch,           // Y
            sin_yaw * cos_pitch, // Z
        );

        transform.translation = self.target + dir * self.radius;
        transform.look_at(self.target, Vec3::Y);
    }
}

/// Spawn the 3D camera as an orbit camera around the origin (Earth)
pub fn setup_camera(mut commands: Commands) {
    let orbit = OrbitCamera::default();

    let mut transform = Transform::default();
    orbit.update_transform(&mut transform);

    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        transform,
        GlobalTransform::default(),
        orbit,
    ));
}

/// Left-mouse drag to orbit (LeoLabs-style trackball)
pub fn orbit_camera(
    mut query: Single<(&mut Transform, &mut OrbitCamera), With<Camera>>,
    settings: Res<CameraSettings>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
) {
    // Only rotate while holding left mouse button
    if !mouse_buttons.pressed(MouseButton::Left) {
        return;
    }

    let (mut transform, mut orbit) = query.into_inner();

    let delta = mouse_motion.delta;

    // Screen X grows to the right; dragging right should spin east,
    // so we increase yaw with +delta.x.
    orbit.yaw += delta.x * settings.rotate_speed;

    // Screen Y grows downward; dragging up gives negative delta.y.
    // We want "drag up" to look north (increase pitch), so subtract.
    orbit.pitch -= delta.y * settings.rotate_speed;

    // Clamp pitch so we don't flip over the poles
    orbit.pitch = orbit
        .pitch
        .clamp(settings.pitch_range.start, settings.pitch_range.end);

    orbit.update_transform(&mut transform);
}

/// Scroll wheel zoom in/out
pub fn zoom_camera(
    mut query: Single<(&mut Transform, &mut OrbitCamera), With<Camera>>,
    settings: Res<CameraSettings>,
    scroll: Res<AccumulatedMouseScroll>,
) {
    let (mut transform, mut orbit) = query.into_inner();

    let scroll_y = scroll.delta.y;
    if scroll_y.abs() == 0.0 {
        return;
    }

    // Smooth zoom: scale radius based on scroll.
    // Positive scroll_y (wheel up) should zoom in => shrink radius.
    let zoom_factor = (1.0 - scroll_y * settings.zoom_speed).max(0.1);
    orbit.radius =
        (orbit.radius * zoom_factor).clamp(settings.radius_range.start, settings.radius_range.end);

    orbit.update_transform(&mut transform);
}
