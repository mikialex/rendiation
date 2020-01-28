#version 450

layout(location = 0) in vec2 v_uv;
layout(location = 1) in vec3 v_world;
layout(location = 2) in vec3 v_normal;

layout(location = 0) out vec4 o_color;

layout(set = 0, binding = 1) uniform texture2D t_Color;
layout(set = 0, binding = 2) uniform sampler s_Color;

void main() {
    vec4 tex = texture(sampler2D(t_Color, s_Color), v_uv);
    float mag = length(v_uv-vec2(0.5));
    o_color = mix(tex, vec4(0.0), mag*mag);

    o_color = vec4(v_world, 1.0) * tex;

    // o_color = vec4((v_normal + vec3(1.)) * 0.5 , 1.);
}