#version 450

layout(location = 0) in vec2 v_uv;
layout(location = 1) in vec3 v_world;
layout(location = 2) in vec3 v_normal;

layout(location = 0) out vec4 o_color;

layout(set = 0, binding = 0) uniform texture2D t_Color;
layout(set = 0, binding = 1) uniform sampler s_Color;

layout(set = 0, binding = 2) uniform Locals {
    float u_frame_id;
};

grain(float time_stamp, float amout){
    fract(10000* sin((gl_FragCoord.x + gl_FragCoord.y * u_frame_id) * pi.y))
}

void main() {
    float amount = 0.01;

    vec4 color = texture(sampler2D(t_Color, s_Color), v_uv);

    float randomIntensity = fract(10000* sin((gl_FragCoord.x + gl_FragCoord.y * u_frame_id) * pi.y));

    amount *= randomIntensity;

    color.rgb += amount;
    o_color = color;
}