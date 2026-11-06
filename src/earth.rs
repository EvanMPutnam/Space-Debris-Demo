//! The Earth: day and night textures lit by the real Sun, an atmosphere
//! shell, and rotation by sidereal time.

use bevy::{
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

use crate::astro::{earth_rotation, sun_direction};
use crate::clock::SimClock;

/// Atmosphere shell radius, in Earth radii.
const ATMOSPHERE_RADIUS: f32 = 1.035;

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct Lighting {
    /// World-space direction toward the Sun; w unused.
    pub sun_direction: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct EarthMaterial {
    #[uniform(0)]
    pub lighting: Lighting,
    #[texture(1)]
    #[sampler(2)]
    pub day: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub night: Handle<Image>,
}

impl Material for EarthMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/earth.wgsl".into()
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct AtmosphereMaterial {
    #[uniform(0)]
    pub lighting: Lighting,
}

impl Material for AtmosphereMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/atmosphere.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
}

#[derive(Component)]
pub struct Earth;

#[derive(Resource)]
struct EarthTextures([Handle<Image>; 2]);

pub struct EarthPlugin;

impl Plugin for EarthPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<EarthMaterial>::default(),
            MaterialPlugin::<AtmosphereMaterial>::default(),
        ))
        .add_systems(Startup, spawn_earth)
        .add_systems(Update, (rotate_earth, update_lighting, generate_mipmaps));
    }
}

fn spawn_earth(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut earth_materials: ResMut<Assets<EarthMaterial>>,
    mut atmosphere_materials: ResMut<Assets<AtmosphereMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let day = asset_server.load("earth.jpg");
    let night = asset_server.load("earth_night.jpg");
    commands.insert_resource(EarthTextures([day.clone(), night.clone()]));

    commands.spawn((
        Name::new("Earth"),
        Earth,
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().uv(256, 128))),
        MeshMaterial3d(earth_materials.add(EarthMaterial {
            lighting: Lighting::default(),
            day,
            night,
        })),
        Transform::default(),
    ));

    commands.spawn((
        Name::new("Atmosphere"),
        Mesh3d(meshes.add(Sphere::new(ATMOSPHERE_RADIUS).mesh().uv(128, 64))),
        MeshMaterial3d(atmosphere_materials.add(AtmosphereMaterial {
            lighting: Lighting::default(),
        })),
        Transform::default(),
    ));
}

fn rotate_earth(clock: Res<SimClock>, mut earth: Single<&mut Transform, With<Earth>>) {
    earth.rotation = earth_rotation(clock.now);
}

fn update_lighting(
    clock: Res<SimClock>,
    mut earth_materials: ResMut<Assets<EarthMaterial>>,
    mut atmosphere_materials: ResMut<Assets<AtmosphereMaterial>>,
) {
    let lighting = Lighting {
        sun_direction: sun_direction(clock.now).extend(0.0),
    };
    for (_, material) in earth_materials.iter_mut() {
        material.lighting = lighting;
    }
    for (_, material) in atmosphere_materials.iter_mut() {
        material.lighting = lighting;
    }
}

/// Bevy does not build mipmaps for JPEGs. Without them a 5400-pixel texture
/// drawn a few hundred pixels wide shimmers, so build them once on load.
fn generate_mipmaps(
    mut events: MessageReader<AssetEvent<Image>>,
    textures: Option<Res<EarthTextures>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(textures) = textures else { return };
    for event in events.read() {
        let AssetEvent::LoadedWithDependencies { id } = event else {
            continue;
        };
        if !textures.0.iter().any(|h| h.id() == *id) {
            continue;
        }
        if let Some(image) = images.get_mut(*id) {
            add_mip_chain(image);
        }
    }
}

/// Appends box-filtered mip levels to an RGBA8 image.
fn add_mip_chain(image: &mut Image) {
    let size = image.texture_descriptor.size;
    let Some(data) = image.data.as_mut() else {
        return;
    };
    if image.texture_descriptor.mip_level_count > 1
        || data.len() != (size.width * size.height * 4) as usize
    {
        return;
    }
    let (mut width, mut height) = (size.width as usize, size.height as usize);
    let mut level_start = 0;
    let mut levels = 1;
    while width > 1 || height > 1 {
        let (next_w, next_h) = ((width / 2).max(1), (height / 2).max(1));
        let mut next = vec![0u8; next_w * next_h * 4];
        for y in 0..next_h {
            for x in 0..next_w {
                for c in 0..4 {
                    let mut sum = 0u32;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let sx = (x * 2 + dx).min(width - 1);
                        let sy = (y * 2 + dy).min(height - 1);
                        sum += u32::from(data[level_start + (sy * width + sx) * 4 + c]);
                    }
                    next[(y * next_w + x) * 4 + c] = (sum / 4) as u8;
                }
            }
        }
        level_start = data.len();
        data.extend_from_slice(&next);
        width = next_w;
        height = next_h;
        levels += 1;
    }
    image.texture_descriptor.mip_level_count = levels;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 16,
        ..default()
    });
}
