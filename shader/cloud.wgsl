struct UniformBufferObject {
    mvp: mat4x4<f32>,
    offset: f32,
    padding: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> ubo: UniformBufferObject;

struct VertexInput {
    @location(0) pos: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

const GRID_SCALE: f32 = 1.0;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let px = model.pos.x;
    let pz = model.pos.z;
    var height = 15.0;

    let snapped_offset = floor(ubo.offset / GRID_SCALE) * GRID_SCALE;
    let world_z = pz + snapped_offset;

    let cloud_density = sin(px * 0.15 + world_z * 0.1) * 0.5
                      + cos(px * 0.05 - world_z * 0.12) * 0.5
                      + sin(px * 0.3 + world_z * 0.02) * 0.25;

    let a = smoothstep(0.3, 0.5, cloud_density);
    out.color = vec4<f32>(1.0, 1.0, 1.0, a);
    height = height + max(0.0, cloud_density - 0.3) * 5.0;

    let offset_remainder = ubo.offset - snapped_offset;
    out.position = ubo.mvp * vec4<f32>(px, height, pz - offset_remainder, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
