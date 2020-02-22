#version 450

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec2 a_uv;

layout(location = 0) out vec2 v_uv;
layout(location = 1) out vec3 v_world;
layout(location = 2) out vec3 v_normal;


layout(set = 0, binding = 0) uniform Locals {
    mat4 u_Transform;
};

void main() {
    gl_Position = u_Transform * vec4(a_position, 1.0);

    v_uv = a_uv;
    v_world = a_position;
    v_normal = a_normal;
}