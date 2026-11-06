//! Screenshots: F12 saves one, and environment variables script a capture
//! for documentation.
//!
//! - `DEMO_SCREENSHOT=out.png` saves after `DEMO_SCREENSHOT_AFTER` seconds
//!   (default 8) and exits.
//! - `DEMO_CAMERA=yaw,pitch,radius` sets the starting view.
//! - `DEMO_RATE_STEPS=n` presses the speed-up key n times at startup.
//! - `DEMO_BREAKUP=n` highlights the nth breakup cloud.
//! - `DEMO_SELECT=norad` selects an object, or `none`.
//! - `DEMO_HIDE=34` hides categories by their legend number.
//! - `DEMO_TRAILS=0` turns trails off.

use bevy::{
    app::AppExit,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};

use crate::camera::OrbitCamera;
use crate::catalog::{Breakup, Catalog};
use crate::objects::DisplaySettings;
use crate::selection::Selection;

#[derive(Resource)]
struct ScriptedCapture {
    path: String,
    after: f32,
    taken_at: Option<f32>,
}

pub struct CapturePlugin;

impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, manual_screenshot)
            .add_systems(PostStartup, apply_environment);
        if let Ok(path) = std::env::var("DEMO_SCREENSHOT") {
            let after = env_parse("DEMO_SCREENSHOT_AFTER").unwrap_or(8.0);
            app.insert_resource(ScriptedCapture {
                path,
                after,
                taken_at: None,
            })
            .add_systems(Update, scripted_screenshot);
        }
    }
}

fn env_parse<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok()?.trim().parse().ok()
}

fn apply_environment(
    mut camera: Single<&mut OrbitCamera>,
    mut settings: ResMut<DisplaySettings>,
    mut selection: ResMut<Selection>,
    mut clock: ResMut<crate::clock::SimClock>,
    catalog: Res<Catalog>,
) {
    if let Ok(view) = std::env::var("DEMO_CAMERA") {
        let values: Vec<f32> = view
            .split(',')
            .filter_map(|v| v.trim().parse().ok())
            .collect();
        if let [yaw, pitch, radius] = values[..] {
            **camera = OrbitCamera::new(yaw, pitch, radius);
        }
    }
    if let Some(n) = env_parse::<usize>("DEMO_BREAKUP") {
        settings.breakup = Breakup::ALL.get(n).copied();
    }
    if let Some(steps) = env_parse::<i32>("DEMO_RATE_STEPS") {
        clock.step_rate(steps);
    }
    if let Some(trails) = env_parse::<u8>("DEMO_TRAILS") {
        settings.trails = trails != 0;
    }
    if let Ok(hidden) = std::env::var("DEMO_HIDE") {
        for slot in hidden.chars().filter_map(|c| c.to_digit(10)) {
            if let Some(visible) = settings.visible.get_mut(slot as usize - 1) {
                *visible = false;
            }
        }
    }
    match std::env::var("DEMO_SELECT").as_deref() {
        Ok("none") => selection.index = None,
        Ok(number) => {
            if let Ok(number) = number.parse::<u32>() {
                selection.index = catalog
                    .entries
                    .iter()
                    .position(|e| e.tle.catalog_number == number);
            }
        }
        Err(_) => {}
    }
}

fn manual_screenshot(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::F12) {
        let path = format!(
            "screenshot-{}.png",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        );
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

fn scripted_screenshot(
    mut commands: Commands,
    time: Res<Time>,
    mut capture: ResMut<ScriptedCapture>,
    mut exit: MessageWriter<AppExit>,
) {
    let elapsed = time.elapsed_secs();
    match capture.taken_at {
        None if elapsed >= capture.after => {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(capture.path.clone()));
            capture.taken_at = Some(elapsed);
        }
        // Give the screenshot a moment to reach the disk.
        Some(taken) if elapsed - taken > 1.5 => {
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}
