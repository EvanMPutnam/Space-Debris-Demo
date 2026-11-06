// Earth surface: daylight texture lit by the Sun, city lights on the night
// side, a warm terminator, sun glint on the oceans, and limb haze.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct Lighting {
    sun_direction: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> lighting: Lighting;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var day_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var day_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var night_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var night_sampler: sampler;

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let sun = normalize(lighting.sun_direction.xyz);
    let to_camera = normalize(view.world_position - in.world_position.xyz);

    let day = textureSample(day_texture, day_sampler, in.uv).rgb;
    let night = textureSample(night_texture, night_sampler, in.uv).rgb;

    let cos_sun = dot(n, sun);
    // 0 on the night side, 1 in daylight, with a soft twilight band.
    let daylight = smoothstep(-0.12, 0.18, cos_sun);

    // The day map has near-black oceans; use that as a water mask.
    let water = 1.0 - smoothstep(0.035, 0.09, luminance(day));

    // Daylight: diffuse with a little wrap so the terminator is not a hard edge.
    let diffuse = clamp((cos_sun + 0.04) / 1.04, 0.0, 1.0);
    // A little earthshine keeps continents faintly visible at night.
    var color = day * (diffuse * 1.6 + 0.035);

    // Oceans pick up a blue tint and a sun glint.
    let half_vector = normalize(sun + to_camera);
    let glint = pow(max(dot(n, half_vector), 0.0), 420.0) * water * smoothstep(0.0, 0.2, cos_sun);
    color += water * vec3<f32>(0.004, 0.014, 0.034) * diffuse;
    color += glint * vec3<f32>(0.4, 0.36, 0.3);

    // Sunset tint along the terminator.
    let twilight = exp(-pow((cos_sun + 0.02) / 0.05, 2.0));
    color += twilight * vec3<f32>(0.30, 0.09, 0.02) * (1.0 - water * 0.6) * 0.25;

    // City lights: keep only the bright, warm pixels of the night map, which
    // also has a faint moonlit base.
    let lights_raw = max(luminance(night) - 0.22, 0.0) / 0.78;
    let warm = clamp(night.r - night.b + 0.35, 0.0, 1.0);
    let lights = pow(lights_raw, 1.35) * warm;
    color += (1.0 - daylight) * lights * vec3<f32>(3.6, 2.3, 1.0);

    // Haze toward the limb, blue on the day side.
    let fresnel = pow(1.0 - max(dot(n, to_camera), 0.0), 5.0);
    color += fresnel * vec3<f32>(0.08, 0.22, 0.6) * smoothstep(-0.1, 0.5, cos_sun);

    return vec4<f32>(color, 1.0);
}
