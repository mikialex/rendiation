use crate::*;

glsl_function!(
  "
vec3 sphericalHarmonics(const in vec3 normal)
{

  const vec3 sph0 = vec3(1.0); // just random values ;)
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
"
);
