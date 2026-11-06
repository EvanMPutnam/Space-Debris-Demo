//! Every tracked object in Earth orbit, propagated live with SGP4.

mod astro;
mod camera;
mod capture;
mod catalog;
mod clock;
mod earth;
mod hud;
mod objects;
mod selection;
mod sky;

use bevy::{prelude::*, window::WindowResolution};

use catalog::{CATALOG_PATH, Catalog};
use clock::SimClock;

fn main() {
    let catalog = Catalog::load(CATALOG_PATH);
    info!(
        "loaded {} objects ({} rejected)",
        catalog.entries.len(),
        catalog.rejected
    );

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Orbital Debris".into(),
                resolution: WindowResolution::new(1600, 960),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(catalog)
        .init_resource::<SimClock>()
        .insert_resource(AmbientLight {
            brightness: 0.0,
            ..default()
        })
        .add_plugins((
            earth::EarthPlugin,
            sky::SkyPlugin,
            objects::ObjectsPlugin,
            selection::SelectionPlugin,
            hud::HudPlugin,
            capture::CapturePlugin,
        ))
        .add_systems(Startup, camera::spawn_camera)
        .add_systems(
            Update,
            (
                clock::clock_controls,
                clock::advance_clock.after(clock::clock_controls),
                camera::control_camera,
            ),
        )
        .run();
}
