#version 450

layout(location = 0) in vec2 v_uv;
layout(location = 1) in vec3 v_world;
layout(location = 2) in vec3 v_normal;

layout(location = 0) out vec4 o_color;

// layout(set = 0, binding = 1) uniform texture2D t_Color;
// layout(set = 0, binding = 2) uniform sampler s_Color;


void main() {
    o_color = vec4(1.0);
}