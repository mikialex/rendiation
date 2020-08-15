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

vec3 sphericalHarmonics(const in vec3 normal )
{

    const vec3 sph0 = vec3(1.0); // just ramdom values ;)
    const vec3 sph1 = vec3(0.3);
    const vec3 sph2 = vec3(-0.2);
    const vec3 sph3 = vec3(0.1);
    const vec3 sph4 = vec3(0.0);
    const vec3 sph5 = vec3(0.0);
    const vec3 sph6 = vec3(0.0);
    const vec3 sph7 = vec3(0.0);
    const vec3 sph8 = vec3(0.0);


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

// fog
// https://docs.microsoft.com/en-us/windows/win32/direct3d9/fog-formulas

const float density = 1.0;
const vec3 fog_color = vec3(0.1, 0.2, 0.3);

const float fog_end = 60.0;
const float fog_start = 30.0;

void main() {
    float distance = length(u_camera_world_position - v_world);

    // distance = distance  / 100000.; // far plane
    // float effect = exp(-density * distance);

    float effect = clamp((fog_end - distance) / (fog_end - fog_start), 0.0, 1.0);

    

    vec3 diffuse = texture(sampler2D(t_Color, s_Color), v_uv).rgb;
    vec3 color = diffuse * sphericalHarmonics(v_normal);

    // color = effect * color + (1.0-effect) * fog_color;
    color = mix(color, fog_color, 1.0 - effect);
    o_color = vec4(color, 1.0);

}