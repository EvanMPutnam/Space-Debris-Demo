//! Background stars, a Milky Way band, and the Sun.

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use crate::astro::{sun_direction, teme_to_world};
use crate::clock::SimClock;

const STAR_COUNT: usize = 16_000;
const SKY_RADIUS: f32 = 600.0;
const SUN_DISTANCE: f32 = 520.0;

#[derive(Component)]
struct Sun;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_sky)
            .add_systems(Update, place_sun);
    }
}

fn spawn_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Stars"),
        Mesh3d(meshes.add(star_mesh())),
        MeshMaterial3d(materials.add(StandardMaterial {
            unlit: true,
            fog_enabled: false,
            ..default()
        })),
        Transform::default(),
    ));

    commands.spawn((
        Name::new("Sun"),
        Sun,
        Mesh3d(meshes.add(Sphere::new(3.2).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: LinearRgba::rgb(60.0, 52.0, 40.0).into(),
            unlit: true,
            ..default()
        })),
        Transform::default(),
    ));
}

fn place_sun(clock: Res<SimClock>, mut sun: Single<&mut Transform, With<Sun>>) {
    sun.translation = sun_direction(clock.now) * SUN_DISTANCE;
}

/// Tiny quads facing the origin, with brightness drawn from a steep power
/// law so a few stars stand out. Half the stars cluster along the galactic
/// plane to suggest the Milky Way.
fn star_mesh() -> Mesh {
    let mut rng = SplitMix64(0x5eed_cafe_f00d_d00d);
    // North galactic pole, in equatorial coordinates (RA 192.86°, Dec 27.13°).
    let pole = equatorial(192.86_f32.to_radians(), 27.13_f32.to_radians());
    let (plane_a, plane_b) = pole.any_orthonormal_pair();

    let mut positions = Vec::with_capacity(STAR_COUNT * 4);
    let mut colors = Vec::with_capacity(STAR_COUNT * 4);
    let mut indices = Vec::with_capacity(STAR_COUNT * 6);

    for i in 0..STAR_COUNT {
        let direction = if i % 5 < 2 {
            random_unit_vector(&mut rng)
        } else {
            let angle = rng.next_f32() * std::f32::consts::TAU;
            let spread = rng.gaussian() * 0.075;
            (plane_a * angle.cos() + plane_b * angle.sin() + pole * spread).normalize()
        };
        let center = direction * SKY_RADIUS;

        // Mostly faint pinpricks, a few bright stars that bloom.
        let bright = rng.next_f32().powf(18.0);
        let brightness = 0.05 + 0.35 * rng.next_f32().powf(4.0) + 3.5 * bright;
        let size = SKY_RADIUS * (0.00045 + 0.0009 * bright);
        let tint = match rng.next_f32() {
            t if t < 0.15 => Vec3::new(0.75, 0.85, 1.25),
            t if t < 0.25 => Vec3::new(1.2, 0.95, 0.7),
            _ => Vec3::ONE,
        };
        let color = (tint * brightness).extend(1.0).to_array();

        let (u, v) = direction.any_orthonormal_pair();
        let base = positions.len() as u32;
        for (du, dv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            positions.push((center + (u * du + v * dv) * size).to_array());
            colors.push(color);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        // Both windings, so the quad shows whichever way it faces.
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

fn equatorial(right_ascension: f32, declination: f32) -> Vec3 {
    let (sin_dec, cos_dec) = declination.sin_cos();
    let teme = [
        f64::from(cos_dec * right_ascension.cos()),
        f64::from(cos_dec * right_ascension.sin()),
        f64::from(sin_dec),
    ];
    teme_to_world(teme).normalize()
}

fn random_unit_vector(rng: &mut SplitMix64) -> Vec3 {
    let z = rng.next_f32() * 2.0 - 1.0;
    let angle = rng.next_f32() * std::f32::consts::TAU;
    let r = (1.0 - z * z).sqrt();
    Vec3::new(r * angle.cos(), z, r * angle.sin())
}

/// A small deterministic generator, so the sky is the same every run.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Standard normal, by Box-Muller.
    fn gaussian(&mut self) -> f32 {
        let u = self.next_f32().max(1e-7);
        let v = self.next_f32();
        (-2.0 * u.ln()).sqrt() * (std::f32::consts::TAU * v).cos()
    }
}
