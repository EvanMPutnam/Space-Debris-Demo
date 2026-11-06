// Atmosphere glow, drawn additively on a shell slightly larger than the Earth.
// Brightness follows the length of the view ray's path through the shell,
// which peaks just outside the limb and fades to zero at the shell's edge.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct Lighting {
    sun_direction: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> lighting: Lighting;

const SHELL_RADIUS: f32 = 1.035;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let sun = normalize(lighting.sun_direction.xyz);
    let ray = normalize(in.world_position.xyz - view.world_position);

    // Distance of the view ray's closest approach to the Earth's center.
    let along = dot(-view.world_position, ray);
    let b = length(view.world_position + ray * along);

    let r2 = SHELL_RADIUS * SHELL_RADIUS;
    let shell = sqrt(max(r2 - b * b, 0.0));
    // Path inside the shell: stops at the surface if the ray hits the Earth.
    var path = 2.0 * shell;
    if b < 1.0 {
        path = shell - sqrt(1.0 - b * b);
    }
    let max_path = 2.0 * sqrt(r2 - 1.0);
    let density = path / max_path;

    let cos_sun = dot(n, sun);
    let lit = smoothstep(-0.2, 0.35, cos_sun);
    let sunset = exp(-pow((cos_sun - 0.04) / 0.1, 2.0));

    let blue = vec3<f32>(0.22, 0.55, 1.35);
    let orange = vec3<f32>(1.4, 0.55, 0.18);
    let color = (blue * lit + orange * sunset * 0.45) * pow(density, 1.8) * 1.1;

    // Premultiplied output with zero alpha blends additively.
    return vec4<f32>(color, 0.0);
}
