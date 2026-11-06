//! On-screen panels: clock, legend, selected-object details, controls, stats.

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use sgp4_rust::Geodetic;

use crate::catalog::{Catalog, Category};
use crate::clock::SimClock;
use crate::objects::{DisplaySettings, PropagationStats};
use crate::selection::Selection;

const PANEL: Color = Color::srgba(0.02, 0.03, 0.06, 0.72);
const BORDER: Color = Color::srgba(0.45, 0.6, 0.9, 0.18);
const TEXT: Color = Color::srgb(0.86, 0.9, 0.97);
const DIM: Color = Color::srgb(0.48, 0.54, 0.64);
const ACCENT: Color = Color::srgb(0.55, 0.78, 1.0);

#[derive(Component)]
struct ClockText;
#[derive(Component)]
struct LegendLabel(Category);
#[derive(Component)]
struct BreakupText;
#[derive(Component)]
struct SelectionPanel;
#[derive(Component)]
struct SelectionTitle;
#[derive(Component)]
struct SelectionBody;
#[derive(Component)]
struct StatsText;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, spawn_hud)
            .add_systems(
                Update,
                (update_clock, update_legend, update_selection, update_stats),
            );
    }
}

fn font(size: f32) -> TextFont {
    TextFont {
        font_size: size,
        ..default()
    }
}

fn panel(node: Node) -> impl Bundle {
    (
        Node {
            padding: UiRect::all(Val::Px(14.0)),
            border: UiRect::all(Val::Px(1.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..node
        },
        BackgroundColor(PANEL),
        BorderColor::all(BORDER),
        BorderRadius::all(Val::Px(10.0)),
    )
}

fn spawn_hud(mut commands: Commands, catalog: Res<Catalog>) {
    // Top left: title, clock, legend.
    commands
        .spawn(panel(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Px(16.0),
            min_width: Val::Px(290.0),
            ..default()
        }))
        .with_children(|p| {
            p.spawn((Text::new("ORBITAL DEBRIS"), font(22.0), TextColor(TEXT)));
            p.spawn((
                Text::new(format!(
                    "{} tracked objects  |  SGP4 in Rust",
                    catalog.entries.len()
                )),
                font(12.0),
                TextColor(DIM),
            ));
            p.spawn((
                Text::default(),
                font(15.0),
                TextColor(ACCENT),
                ClockText,
                Node {
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
            ));
            for (slot, category) in Category::ALL.into_iter().enumerate() {
                p.spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    ..default()
                })
                .with_children(|row| {
                    let c = category.color();
                    let peak = c.red.max(c.green).max(c.blue);
                    row.spawn((
                        Node {
                            width: Val::Px(10.0),
                            height: Val::Px(10.0),
                            ..default()
                        },
                        BackgroundColor(Color::LinearRgba(LinearRgba::rgb(
                            c.red / peak,
                            c.green / peak,
                            c.blue / peak,
                        ))),
                        BorderRadius::all(Val::Px(5.0)),
                    ));
                    row.spawn((
                        Text::new(format!(
                            "[{}] {:<18}{:>6}",
                            slot + 1,
                            category.label(),
                            catalog.count(category)
                        )),
                        font(14.0),
                        TextColor(TEXT),
                        LegendLabel(category),
                    ));
                });
            }
            p.spawn((
                Text::default(),
                font(12.5),
                TextColor(Color::srgb(1.0, 0.55, 0.9)),
                BreakupText,
            ));
        });

    // Right: selected object.
    commands
        .spawn((
            panel(Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                right: Val::Px(16.0),
                width: Val::Px(320.0),
                ..default()
            }),
            SelectionPanel,
        ))
        .with_children(|p| {
            p.spawn((Text::default(), font(18.0), TextColor(TEXT), SelectionTitle));
            p.spawn((Text::default(), font(13.5), TextColor(TEXT), SelectionBody));
        });

    // Bottom left: controls.
    commands
        .spawn(panel(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(16.0),
            left: Val::Px(16.0),
            ..default()
        }))
        .with_child((
            Text::new(
                "drag  rotate      scroll  zoom      click  select      esc  clear\n\
                 space  pause      < >  speed      R  reverse      N  now\n\
                 1-4  categories   T  trails       B  breakup clouds",
            ),
            font(12.0),
            TextColor(DIM),
        ));

    // Bottom right: performance.
    commands
        .spawn(panel(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(16.0),
            right: Val::Px(16.0),
            ..default()
        }))
        .with_child((Text::default(), font(12.0), TextColor(DIM), StatsText));
}

fn update_clock(clock: Res<SimClock>, mut text: Single<&mut Text, With<ClockText>>) {
    let time = clock
        .now
        .to_datetime()
        .map(|t| t.format("%Y-%m-%d  %H:%M:%S UTC").to_string())
        .unwrap_or_default();
    let speed = if clock.paused {
        "paused".to_string()
    } else {
        let direction = if clock.reversed { "reverse " } else { "" };
        format!("{direction}{}x real time", clock.rate().abs())
    };
    text.0 = format!("{time}\n{speed}");
}

fn update_legend(
    settings: Res<DisplaySettings>,
    mut labels: Query<(&LegendLabel, &mut TextColor)>,
    mut breakup: Single<&mut Text, With<BreakupText>>,
    catalog: Res<Catalog>,
) {
    if !settings.is_changed() {
        return;
    }
    for (label, mut color) in &mut labels {
        color.0 = if settings.shows(label.0) {
            TEXT
        } else {
            DIM.with_alpha(0.5)
        };
    }
    breakup.0 = match settings.breakup {
        Some(b) => {
            let count = catalog
                .entries
                .iter()
                .filter(|e| e.breakup == Some(b))
                .count();
            format!("\n{}\n{count} tracked fragments", b.description())
        }
        None => String::new(),
    };
}

fn update_selection(
    selection: Res<Selection>,
    catalog: Res<Catalog>,
    clock: Res<SimClock>,
    mut panel: Single<&mut Visibility, With<SelectionPanel>>,
    mut title: Single<&mut Text, (With<SelectionTitle>, Without<SelectionBody>)>,
    mut body: Single<&mut Text, (With<SelectionBody>, Without<SelectionTitle>)>,
) {
    let Some(entry) = selection.index.map(|i| &catalog.entries[i]) else {
        **panel = Visibility::Hidden;
        return;
    };
    **panel = Visibility::Inherited;
    let satellite = &entry.satellite;
    let tle = &entry.tle;
    title.0 = entry.name().to_string();

    let state = satellite.propagate_to(clock.now);
    let live = match state {
        Ok(state) => {
            let ground = Geodetic::from_teme(state.position, clock.now);
            let speed = state.velocity.iter().map(|v| v * v).sum::<f64>().sqrt();
            format!(
                "Altitude     {:>9.1} km\n\
                 Latitude     {:>9.2} deg\n\
                 Longitude    {:>9.2} deg\n\
                 Speed        {:>9.3} km/s",
                ground.altitude_km,
                ground.latitude.to_degrees(),
                ground.longitude.to_degrees(),
                speed
            )
        }
        Err(error) => format!("SGP4 error: {error}"),
    };
    let age_days = clock.now.minutes_since(satellite.epoch()) / 1440.0;

    body.0 = format!(
        "NORAD {}   COSPAR {}\n{}{}\n\n{live}\n\n\
         Period       {:>9.1} min\n\
         Perigee      {:>9.0} km\n\
         Apogee       {:>9.0} km\n\
         Inclination  {:>9.2} deg\n\
         Eccentricity {:>9.5}\n\
         TLE age      {:>9.1} days",
        tle.catalog_number,
        if tle.international_designator.is_empty() {
            "-"
        } else {
            &tle.international_designator
        },
        entry.category.label(),
        entry
            .breakup
            .map(|b| format!("\n{}", b.description()))
            .unwrap_or_default(),
        satellite.period_minutes(),
        satellite.perigee_altitude_km(),
        satellite.apogee_altitude_km(),
        tle.inclination_deg,
        tle.eccentricity,
        age_days,
    );
}

fn update_stats(
    diagnostics: Res<DiagnosticsStore>,
    stats: Res<PropagationStats>,
    catalog: Res<Catalog>,
    mut text: Single<&mut Text, With<StatsText>>,
) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    text.0 = format!(
        "{fps:>5.0} fps\nSGP4  {:>5} objects in {:.1} ms\n{} decayed or invalid",
        catalog.entries.len(),
        stats.millis,
        stats.failed + catalog.rejected,
    );
}
