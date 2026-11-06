//! One entity per catalog object, propagated in parallel every frame.

use std::time::Instant;

use bevy::prelude::*;

use crate::astro::teme_to_world;
use crate::catalog::{Breakup, Catalog, Category};
use crate::clock::SimClock;

/// On-screen size of an object, as a fraction of its distance from the camera.
const ANGULAR_SIZE: f32 = 0.0009;
/// Trails show where an object was this many simulated minutes ago.
const TRAIL_MINUTES: f32 = 1.5;

#[derive(Component)]
pub struct Tracked {
    pub index: usize,
    pub category: Category,
    pub breakup: Option<Breakup>,
}

/// World-space state from the latest propagation.
#[derive(Component, Default)]
pub struct Motion {
    pub position: Vec3,
    /// Earth radii per minute (`teme_to_world` of km/s, times 60 s).
    pub velocity: Vec3,
    /// False when SGP4 returned an error (usually a decayed orbit).
    pub valid: bool,
}

#[derive(Resource)]
pub struct DisplaySettings {
    pub visible: [bool; 4],
    pub trails: bool,
    /// When set, this breakup's debris is lit and everything else is dimmed.
    pub breakup: Option<Breakup>,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            visible: [true; 4],
            trails: true,
            breakup: None,
        }
    }
}

impl DisplaySettings {
    pub fn shows(&self, category: Category) -> bool {
        self.visible[category_slot(category)]
    }
}

fn category_slot(category: Category) -> usize {
    Category::ALL.iter().position(|c| *c == category).unwrap()
}

#[derive(Resource, Default)]
pub struct PropagationStats {
    pub millis: f32,
    pub failed: usize,
}

#[derive(Resource)]
struct ObjectMaterials {
    by_category: [Handle<StandardMaterial>; 4],
    dimmed: Handle<StandardMaterial>,
    highlighted: Handle<StandardMaterial>,
}

pub struct ObjectsPlugin;

impl Plugin for ObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DisplaySettings>()
            .init_resource::<PropagationStats>()
            .add_systems(Startup, spawn_objects)
            .add_systems(
                Update,
                (
                    display_controls,
                    apply_breakup_highlight.run_if(resource_changed::<DisplaySettings>),
                    propagate_objects.after(crate::clock::advance_clock),
                    draw_trails.after(propagate_objects),
                ),
            );
    }
}

fn unlit(color: LinearRgba) -> StandardMaterial {
    StandardMaterial {
        base_color: color.into(),
        unlit: true,
        ..default()
    }
}

fn spawn_objects(
    mut commands: Commands,
    catalog: Res<Catalog>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
    let object_materials = ObjectMaterials {
        by_category: Category::ALL.map(|c| materials.add(unlit(c.color()))),
        dimmed: materials.add(unlit(LinearRgba::rgb(0.12, 0.13, 0.16))),
        highlighted: materials.add(unlit(LinearRgba::rgb(3.5, 0.5, 2.8))),
    };

    commands.spawn_batch(
        catalog
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                (
                    Tracked {
                        index,
                        category: entry.category,
                        breakup: entry.breakup,
                    },
                    Motion::default(),
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(
                        object_materials.by_category[category_slot(entry.category)].clone(),
                    ),
                    Transform::default(),
                    Visibility::Hidden,
                )
            })
            .collect::<Vec<_>>(),
    );
    commands.insert_resource(object_materials);
}

/// SGP4 for every object at the clock's time. `Satellite::propagate_to`
/// takes `&self`, so this runs across all cores with no locking.
pub fn propagate_objects(
    clock: Res<SimClock>,
    catalog: Res<Catalog>,
    settings: Res<DisplaySettings>,
    camera: Single<&GlobalTransform, With<Camera3d>>,
    mut stats: ResMut<PropagationStats>,
    mut objects: Query<(&Tracked, &mut Motion, &mut Transform, &mut Visibility)>,
) {
    let start = Instant::now();
    let now = clock.now;
    let camera_position = camera.translation();

    objects
        .par_iter_mut()
        .for_each(|(tracked, mut motion, mut transform, mut visibility)| {
            let entry = &catalog.entries[tracked.index];
            let shown = match entry.satellite.propagate_to(now) {
                Ok(state) => {
                    motion.position = teme_to_world(state.position);
                    motion.velocity = teme_to_world(state.velocity) * 60.0;
                    motion.valid = true;
                    transform.translation = motion.position;
                    transform.scale =
                        Vec3::splat(camera_position.distance(motion.position) * ANGULAR_SIZE);
                    settings.shows(tracked.category)
                }
                Err(_) => {
                    motion.valid = false;
                    false
                }
            };
            visibility.set_if_neq(if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        });

    stats.millis = start.elapsed().as_secs_f32() * 1000.0;
    stats.failed = objects.iter().filter(|(_, m, _, _)| !m.valid).count();
}

fn draw_trails(
    settings: Res<DisplaySettings>,
    mut gizmos: Gizmos,
    objects: Query<(&Tracked, &Motion, &ViewVisibility)>,
) {
    if !settings.trails {
        return;
    }
    for (tracked, motion, visible) in &objects {
        if !visible.get() {
            continue;
        }
        let lit = settings.breakup.is_none_or(|b| tracked.breakup == Some(b));
        if !lit {
            continue;
        }
        let color = if settings.breakup.is_some() {
            LinearRgba::rgb(1.6, 0.25, 1.3)
        } else {
            tracked.category.color() * 0.16
        };
        let tail = motion.position - motion.velocity * TRAIL_MINUTES;
        gizmos.line_gradient(motion.position, tail, color, LinearRgba::NONE);
    }
}

fn display_controls(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<DisplaySettings>) {
    let toggles = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ];
    for (slot, key) in toggles.into_iter().enumerate() {
        if keys.just_pressed(key) {
            settings.visible[slot] = !settings.visible[slot];
        }
    }
    if keys.just_pressed(KeyCode::KeyT) {
        settings.trails = !settings.trails;
    }
    if keys.just_pressed(KeyCode::KeyB) {
        let next = match settings.breakup {
            None => Some(Breakup::ALL[0]),
            Some(current) => {
                let i = Breakup::ALL.iter().position(|b| *b == current).unwrap();
                Breakup::ALL.get(i + 1).copied()
            }
        };
        settings.breakup = next;
    }
}

fn apply_breakup_highlight(
    settings: Res<DisplaySettings>,
    materials: Res<ObjectMaterials>,
    mut objects: Query<(&Tracked, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    for (tracked, mut material) in &mut objects {
        let wanted = match settings.breakup {
            None => &materials.by_category[category_slot(tracked.category)],
            Some(b) if tracked.breakup == Some(b) => &materials.highlighted,
            Some(_) => &materials.dimmed,
        };
        if material.0 != *wanted {
            material.0 = wanted.clone();
        }
    }
}
