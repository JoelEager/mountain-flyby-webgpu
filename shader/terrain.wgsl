// Uniform buffer object containing the MVP matrix and time offset
struct UniformBufferObject {
    mvp: mat4x4<f32>,
    offset: f32,
};

@group(0) @binding(0)
var<uniform> ubo: UniformBufferObject;

struct VertexInput {
    @location(0) pos: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

const GRID_SCALE: f32 = 0.5;
const VALLEY_WIDTH: f32 = 24.0;
const MOUNTAIN_SCALE: f32 = 9.0;

fn get_height(px: f32, pz: f32, world_z: f32) -> f32 {
    let modified_world_z = world_z + sin(px) * 5.0;
    var height = sin(modified_world_z * 0.2) * 2.0 + cos(modified_world_z * 0.5) * 1.0;

    let modified_px = px + sin(world_z * 0.3) * 0.6 + cos(world_z * 0.8) * 1.2;
    let mountain_ratio = (abs(modified_px) - VALLEY_WIDTH) / VALLEY_WIDTH;
    let ridge_factor = max(1.0 - abs(mountain_ratio), 0.0);
    height = height * ridge_factor + ridge_factor * MOUNTAIN_SCALE;

    height = height + sin(px * 1.5 + world_z * 0.8) * 0.13;
    height = height + cos(px * 1.0 - world_z * 2.0) * 0.09;
    height = height - sin(-px * 0.9 + world_z * 1.3) * 0.1;

    return height;
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let px = model.pos.x;
    let pz = model.pos.z;

    let snapped_offset = floor(ubo.offset / GRID_SCALE) * GRID_SCALE;
    let world_z = pz + snapped_offset;

    let height = get_height(px, pz, world_z);

    let snow_color = vec3<f32>(0.9, 0.9, 0.95);
    let rock_color = vec3<f32>(0.5, 0.45, 0.45);

    let noise = (sin(px * 1.5 + world_z * 0.8) + cos(px * 3.0 - world_z * 2.0)) * 0.08;
    let grass_color = vec3<f32>(0.2 + noise, 0.5 + noise, 0.2 + noise);

    var color = mix(grass_color, rock_color, smoothstep(6.0, 7.0, height));
    color = mix(color, snow_color, smoothstep(9.0, 10.0, height));

    let eps = 0.1;
    let h_x = get_height(px + eps, pz, world_z);
    let h_z = get_height(px, pz + eps, world_z + eps);

    let p = vec3<f32>(px, height, pz);
    let p_x = vec3<f32>(px + eps, h_x, pz);
    let p_z = vec3<f32>(px, h_z, pz + eps);

    let normal = normalize(cross(p_z - p, p_x - p));

    let lightDir = normalize(vec3<f32>(-1.0, 1.0, -0.5));
    let diffuse = max(dot(normal, lightDir), 0.0);
    let ambient = 0.3;

    color = color * (diffuse + ambient);

    let dist = length(vec2<f32>(px, pz));
    let fog_factor = smoothstep(100.0, 500.0, dist);
    let fog_color = vec3<f32>(0.3, 0.5, 0.8);

    out.color = mix(color, fog_color, fog_factor);

    let offset_remainder = ubo.offset - snapped_offset;
    out.position = ubo.mvp * vec4<f32>(px, height, pz - offset_remainder, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let linear_color = pow(in.color, vec3<f32>(2.2, 2.2, 2.2));
    return vec4<f32>(linear_color, 1.0);
}
