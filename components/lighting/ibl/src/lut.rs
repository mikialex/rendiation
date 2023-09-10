/// https://github.com/AnalyticalGraphicsInc/cesium/blob/master/Source/Shaders/BrdfLutGeneratorFS.glsl
pub fn integrate_brdf(roughness: Node<f32>, n_dot_v: Node<f32>) -> Vec2<f32> {
  todo!()
}

// vec2 integrateBRDF(const in sampler2D hammersleySampler, const in float roughness, const in float
// nDotV) {   float roughness2 = roughness * roughness;
//   float roughness4 = roughness2 * roughness2;
//   vec3 normal = vec3(0.0, 0.0, 1.0);
//   vec3 view = vec3(sqrt(1.0 - nDotV * nDotV), 0.0, nDotV);

//   vec2 res = vec2(0.0, 0.0);

//   for (int i = 0; i < NUM_SAMPLES; ++i) {
//     vec2 xi = hammersley(hammersleySampler, i, NUM_SAMPLES);
//     vec3 halfVectorTangentSpace = importanceSampleGGX(xi, roughness4);
//     vec3 halfVector = tangentToWorldSpace(halfVectorTangentSpace, normal);
//     vec3 light = 2.0 * dot(view, halfVector) * halfVector - view;
//     float nDotL = max(light.z, 0.0);
//     float nDotH = max(halfVector.z, 0.0);
//     float vDotH = max(dot(view, halfVector), 0.0);

//     float g = smithHeightCorrelatedGeometric(nDotL, nDotV, roughness4);
//     float gVis = max(g * vDotH / (nDotH * nDotV), 0.0);
//     float fC = pow(1.0 - vDotH, 5.0);
//     res.x += (1.0 - fC) * gVis;
//     res.y += fC * gVis;
//   }
//   return res / float(NUM_SAMPLES);
// }
