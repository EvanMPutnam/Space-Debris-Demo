//! Simulation time, which can run faster than real time, pause, or reverse.

use bevy::prelude::*;
use chrono::Utc;
use sgp4_rust::JulianDate;

/// Speeds the user can step through, in simulated seconds per real second.
const RATES: [f64; 9] = [1.0, 10.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0, 3600.0];
const DEFAULT_RATE_INDEX: usize = 3;

#[derive(Resource)]
pub struct SimClock {
    pub now: JulianDate,
    rate_index: usize,
    pub reversed: bool,
    pub paused: bool,
}

impl Default for SimClock {
    fn default() -> Self {
        Self {
            now: Utc::now().into(),
            rate_index: DEFAULT_RATE_INDEX,
            reversed: false,
            paused: false,
        }
    }
}

impl SimClock {
    /// Simulated seconds per real second, negative when reversed.
    pub fn rate(&self) -> f64 {
        let rate = RATES[self.rate_index];
        if self.reversed { -rate } else { rate }
    }

    /// Moves `steps` places up (or down, if negative) the list of speeds.
    pub fn step_rate(&mut self, steps: i32) {
        let index = self.rate_index as i32 + steps;
        self.rate_index = index.clamp(0, RATES.len() as i32 - 1) as usize;
    }
}

pub fn advance_clock(time: Res<Time>, mut clock: ResMut<SimClock>) {
    if !clock.paused {
        let minutes = time.delta_secs_f64() * clock.rate() / 60.0;
        clock.now = clock.now.add_minutes(minutes);
    }
}

pub fn clock_controls(keys: Res<ButtonInput<KeyCode>>, mut clock: ResMut<SimClock>) {
    if keys.just_pressed(KeyCode::Space) {
        clock.paused = !clock.paused;
    }
    if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::Period) {
        clock.step_rate(1);
    }
    if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::Comma) {
        clock.step_rate(-1);
    }
    if keys.just_pressed(KeyCode::KeyR) {
        clock.reversed = !clock.reversed;
    }
    if keys.just_pressed(KeyCode::KeyN) {
        clock.now = Utc::now().into();
    }
}
