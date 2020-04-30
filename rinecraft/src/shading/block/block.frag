#version 450

layout(location = 0) in vec2 v_uv;
layout(location = 1) in vec3 v_world;
layout(location = 2) in vec3 v_normal;

layout(location = 0) out vec4 o_color;

layout(set = 0, binding = 1) uniform texture2D t_Color;
layout(set = 0, binding = 2) uniform sampler s_Color;
layout(set = 0, binding = 3) uniform Locals {
    vec3 u_camera_world_position;
};

const vec3 sph0 = vec3(1.0); // just ramdom values ;)
const vec3 sph1 = vec3(0.3);
const vec3 sph2 = vec3(-0.2);
const vec3 sph3 = vec3(0.1);
const vec3 sph4 = vec3(0.0);
const vec3 sph5 = vec3(0.0);
const vec3 sph6 = vec3(0.0);
const vec3 sph7 = vec3(0.0);
const vec3 sph8 = vec3(0.0);

vec3 sphericalHarmonics(const in vec3 normal )
{
    float x = normal.x;
    float y = normal.y;
    float z = normal.z;

    vec3 result = (
        sph0 +

        sph1 * y +
        sph2 * z +
        sph3 * x +

        sph4 * y * x +
        sph5 * y * z +
        sph6 * (3.0 * z * z - 1.0) +
        sph7 * (z * x) +
        sph8 * (x*x - y*y)
    );

    return max(result, vec3(0.0));
}

// vec3 sph

void main() {
    vec3 diffuse = texture(sampler2D(t_Color, s_Color), v_uv).rgb;
    o_color = vec4(diffuse * sphericalHarmonics(v_normal), 1.0);

}