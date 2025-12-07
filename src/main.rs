use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};
mod camera;
mod debris;
mod loader;

use crate::debris::{setup_debris_field, setup_simulation_time, update_debris_positions};
use camera::{CameraSettings, orbit_camera, setup_camera, zoom_camera};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<CameraSettings>()
        .add_systems(
            Startup,
            (
                setup_scene,
                show_instructions,
                setup_camera,
                setup_debris_field,
                setup_simulation_time,
            ),
        )
        .add_systems(Update, (orbit_camera, zoom_camera, update_debris_positions))
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let earth_mesh = meshes.add(Sphere::new(1.0).mesh().uv(128, 64));
    let earth_texture: Handle<Image> = asset_server.load("earth.jpg");

    let earth_material = materials.add(StandardMaterial {
        base_color_texture: Some(earth_texture),
        unlit: true,
        ..default()
    });

    commands.spawn((
        Name::new("Earth"),
        Mesh3d(earth_mesh),
        MeshMaterial3d(earth_material),
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -FRAC_PI_2,
            0.0,
            2.0 * PI - FRAC_PI_2,
        )),
        GlobalTransform::default(),
    ));
}

fn show_instructions(mut commands: Commands) {
    commands.spawn((
        Name::new("Instructions"),
        Text::new(
            "Left mouse: drag to orbit\n\
             Scroll wheel: zoom",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}
