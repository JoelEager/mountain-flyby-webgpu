#version 400
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// Input interpolated color received from the vertex shader
layout (location = 0) in vec3 o_color;

// Output color for the fragment
layout (location = 0) out vec4 uFragColor;

// The main function executes once per fragment (pixel).
// It's a simple fragment shader which passes through the interpolated
// color received from the vertex shader to color the pixel.
void main() {
    uFragColor = vec4(o_color, 1.0);
}
