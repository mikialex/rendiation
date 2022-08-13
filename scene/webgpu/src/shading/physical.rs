use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct PhysicalShading {
  pub diffuse: Vec3<f32>,
  pub specular: Vec3<f32>,
  pub roughness: f32,
}

both!(ColorChannel, Vec3<f32>);
both!(SpecularChannel, Vec3<f32>);
both!(RoughnessChannel, f32);

impl LightableSurfaceShading for PhysicalShading {
  fn construct(builder: &mut ShaderGraphFragmentBuilder) -> ExpandedNode<Self> {
    ExpandedNode::<Self> {
      diffuse: builder.query_or_insert_default::<ColorChannel>().get(),
      specular: builder.query_or_insert_default::<SpecularChannel>().get(),
      roughness: builder.query_or_insert_default::<RoughnessChannel>().get(),
    }
  }

  fn compute_lighting(
    self_node: &ExpandedNode<Self>,
    direct_light: &ExpandedNode<ShaderIncidentLight>,
    ctx: &ExpandedNode<ShaderLightingGeometricCtx>,
  ) -> ExpandedNode<ShaderLightingResult> {
    physical_shading(
      direct_light.construct(),
      ctx.construct(),
      self_node.construct(),
    )
    .expand()
  }
}

wgsl_function!(
  fn physical_shading(
    directLight: ShaderIncidentLight,
    geometry: ShaderLightingGeometricCtx,
    shading: PhysicalShading,
  ) -> ShaderLightingResult {
    let nDotL = biasNDotL(dot(-directLight.direction, geometry.normal));
    if nDotL == 0.0 {
      return;
    }
    let directDiffuseBRDF = evaluateBRDFDiffuse(material.diffuse);
    let directSpecularBRDF = evaluateBRDFSpecular(
      geometry.viewDir,
      -directLight.direction,
      geometry.normal,
      material.specular,
      material.roughness,
    );
    reflectedLight.directDiffuse += directLight.color * directDiffuseBRDF * nDotL;
    reflectedLight.directSpecular += directLight.color * directSpecularBRDF * nDotL;
  }
);

wgsl_function!(
  // Reduces shadow mapping artifacts near tangent
  fn biasNDotL(nDotL: f32) -> f32 {
    return clamp(nDotL * 1.08 - 0.08, 0.0, 1.0);
  }
);

wgsl_function!(
  // https://www.cs.cornell.edu/~srm/publications/EGSR07-btdf.pdf
  fn D_GGX(NoH: f32, roughness4: f32) -> f32 {
    let d = (NoH * roughness4 - NoH) * NoH + 1.0;
    return roughness4 / (PI * d * d);
  }
);

wgsl_function!(
  // NOTE: Basically same as
  // https://de45xmedrsdbp.cloudfront.net/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
  // However, calculate a F90 instead of using 1.0 directlly
  fn fresnel(vDotH: f32, f0: f32) -> f32 {
    let fc = pow(1.0 - vDotH, 5.0);
    let f90 = clamp(f0 * 50.0, 0.0, 1.0);
    return f90 * fc + f0 * (1.0 - fc);
  }
);

wgsl_function!(
  // NOTE: the microfacet model G part
  // TODO: Need reference for this part (1.0 or 0.5)
  fn visibility(nDotL: f32, nDotV: f32, roughness4: f32) -> f32 {
    let Vis_SmithV = nDotV + sqrt(nDotV * (nDotV - nDotV * roughness4) + roughness4);
    let Vis_SmithL = nDotL + sqrt(nDotL * (nDotL - nDotL * roughness4) + roughness4);
    return 1.0 / (Vis_SmithV * Vis_SmithL);
  }
);

wgsl_function!(
  fn evaluateBRDFDiffuse(diffuseColor: vec3<f32>) -> vec3<f32> {
    return INVERSE_PI * diffuseColor;
  }
);

wgsl_function!(
  fn evaluateBRDFSpecular(
    V: vec3<f32>,
    L: vec3<f32>,
    N: vec3<f32>,
    specularColor: vec3<f32>,
    roughness: f32,
  ) -> vec3<f32> {
    let H = normalize(L + V);
    let nDotL = max(dot(L, N), 0.0);
    let nDotV = max(EPSILON_SHADING, dot(N, V));
    let nDotH = max(EPSILON_SHADING, dot(N, H));
    let vDotH = max(EPSILON_SHADING, dot(V, H));
    let roughness2 = roughness * roughness;
    let roughness4 = roughness2 * roughness2;

    let f = fresnel(vDotH, specularColor);
    let d = max(D_GGX(nDotH, roughness4), 0.0);
    let g = max(visibility(nDotL, nDotV, roughness4), 0.0);

    return f * (d * g);
  }
);
