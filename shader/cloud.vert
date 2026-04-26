#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// Input vertex position
layout (location = 0) in vec4 pos;

// Uniform buffer object containing the Model-View-Projection (MVP) matrix and the time-based offset
layout (binding = 0) uniform UniformBufferObject {
    mat4 mvp;
    float offset;
} ubo;

// Output color to be passed to the fragment shader
layout (location = 0) out vec4 o_color;

// The space between mesh vertices. Must match the value used in landscape.rs
const float GRID_SCALE = 1.0;

void main() {
    float px = pos.x;
    float pz = pos.z;
    float height = 15.0;

    // Move world z in steps that match GRID_SCALE so the noise stays aligned to the vertices
    float snapped_offset = floor(ubo.offset / GRID_SCALE) * GRID_SCALE;
    float world_z = pz + snapped_offset;

    // Cloud rendering logic
    float cloud_density = sin(px * 0.15 + world_z * 0.1) * 0.5
                        + cos(px * 0.05 - world_z * 0.12) * 0.5
                        + sin(px * 0.3 + world_z * 0.02) * 0.25;

    o_color = vec4(1.0, 1.0, 1.0, 0.0);
    o_color.a = smoothstep(0.3, 0.5, cloud_density);
    height += max(0.0, cloud_density - 0.3) * 5.0;

    // Calculate the z remainder to smooth out the stepped cloud movement
    float offset_remainder = ubo.offset - snapped_offset; // Always between 0.0 and GRID_SCALE

    gl_Position = ubo.mvp * vec4(px, height, pz - offset_remainder, 1.0);
}
