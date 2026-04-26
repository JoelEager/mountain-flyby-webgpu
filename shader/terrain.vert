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
layout (location = 0) out vec3 o_color;

// The space between mesh vertices. Must match the value used in landscape.rs
const float GRID_SCALE = 0.5;

// Constants to control terrain height generation
const float VALLEY_WIDTH = 24.0;                        // Must be smaller than terrain mesh width * GRID_SCALE / 2
const float MOUNTAIN_SCALE = 9.0;

// Calculate height of a given vertex
float get_height(float px, float pz, float world_z) {
    // Base height from Perlin-like noise (simplified with trig functions)
    float modified_world_z = world_z + sin(px) * 5.0;                       // Blend some x into the base noise so the two mountain ridges don't look so similar
    float height = sin(modified_world_z * 0.2) * 2.0 + cos(modified_world_z * 0.5) * 1.0;

    // Add height to form the mountain ridge
    float modified_px = px + sin(world_z * 0.3) * 0.6 + cos(world_z * 0.8) * 1.2;   // Apply some world z to x so the center of the valley varies
    float mountain_ratio = (abs(modified_px) - VALLEY_WIDTH) / VALLEY_WIDTH;        // closer to 0 -> mountains taller
    float ridge_factor = max(1.0 - abs(mountain_ratio), 0.0);
    height = height * ridge_factor + ridge_factor * MOUNTAIN_SCALE;                 // Increase the amount of noise height when the mountain is taller

    // Add details
    height += sin(px * 1.5 + world_z * 0.8) * 0.13;
    height += cos(px * 1.0 - world_z * 2.0) * 0.09;
    height -= sin(-px * 0.9 + world_z * 1.3) * 0.1;

    return height;
}

// The main function executes once per vertex.
// It computes procedural height based on world position and time offset,
// updates the vertex color, and transforms the position using the MVP matrix.
void main() {
    float px = pos.x;
    float pz = pos.z;

    // Move world z in steps that match GRID_SCALE so the noise stays aligned to the vertices
    float snapped_offset = floor(ubo.offset / GRID_SCALE) * GRID_SCALE;
    float world_z = pz + snapped_offset;

    // Mountain terrain logic
    float height = get_height(px, pz, world_z);

    // Height-based coloring
    vec3 snow_color = vec3(0.9, 0.9, 0.95);
    vec3 rock_color = vec3(0.5, 0.45, 0.45);

    float noise = (sin(px * 1.5 + world_z * 0.8) + cos(px * 3.0 - world_z * 2.0)) * 0.08;
    vec3 grass_color = vec3(0.2 + noise, 0.5 + noise, 0.2 + noise);

    o_color = mix(grass_color, rock_color, smoothstep(6.0, 7.0, height));
    o_color = mix(o_color, snow_color, smoothstep(9.0, 10.0, height));

    // Calculate normals using approximate gradient
    float eps = 0.1;
    float h_x = get_height(px + eps, pz, world_z);
    float h_z = get_height(px, pz + eps, world_z + eps);

    vec3 p = vec3(px, height, pz);
    vec3 p_x = vec3(px + eps, h_x, pz);
    vec3 p_z = vec3(px, h_z, pz + eps);

    vec3 normal = normalize(cross(p_z - p, p_x - p));

    // Calculate simple directional lighting
    vec3 lightDir = normalize(vec3(-1.0, 1.0, -0.5));
    float diffuse = max(dot(normal, lightDir), 0.0);
    float ambient = 0.3;

    o_color = o_color * (diffuse + ambient);

    // Calculate horizon fog
    float dist = length(vec2(px, pz));
    float fog_factor = smoothstep(100.0, 500.0, dist);
    vec3 fog_color = vec3(0.3, 0.5, 0.8);

    o_color = mix(o_color, fog_color, fog_factor);

    // Calculate the z remainder to smooth out the stepped terrain movement
    float offset_remainder = ubo.offset - snapped_offset; // Always between 0.0 and GRID_SCALE

    gl_Position = ubo.mvp * vec4(px, height, pz - offset_remainder, 1.0);
}
