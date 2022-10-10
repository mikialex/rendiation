use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderPhysicalShading {
  pub diffuse: Vec3<f32>,
  pub specular: Vec3<f32>,
  pub roughness: f32,
}

both!(ColorChannel, Vec3<f32>);
both!(SpecularChannel, Vec3<f32>);
both!(RoughnessChannel, f32);

pub struct PhysicalShading;

impl LightableSurfaceShading for PhysicalShading {
  type ShaderStruct = ShaderPhysicalShading;
  fn construct_shading(builder: &mut ShaderGraphFragmentBuilder) -> ENode<Self::ShaderStruct> {
    ENode::<Self::ShaderStruct> {
      diffuse: builder.query_or_insert_default::<ColorChannel>(),
      specular: builder.query_or_insert_default::<SpecularChannel>(),
      roughness: builder.query_or_insert_default::<RoughnessChannel>(),
    }
  }

  fn compute_lighting_by_incident(
    self_node: &ENode<Self::ShaderStruct>,
    direct_light: &ENode<ShaderIncidentLight>,
    ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    physical_shading(
      direct_light.construct(),
      ctx.construct(),
      self_node.construct(),
    )
    .expand()
  }
}

wgsl_fn!(
  fn physical_shading(
    directLight: ShaderIncidentLight,
    geometry: ShaderLightingGeometricCtx,
    shading: ShaderPhysicalShading,
  ) -> ShaderLightingResult {
    var result: ShaderLightingResult;
    let nDotL = biasNDotL(dot(-directLight.direction, geometry.normal));
    if nDotL == 0.0 {
      return result;
    }
    let directDiffuseBRDF = evaluateBRDFDiffuse(shading.diffuse);
    let directSpecularBRDF = evaluateBRDFSpecular(
      geometry.view_dir,
      -directLight.direction,
      geometry.normal,
      shading.specular,
      shading.roughness,
    );
    result.diffuse += directLight.color * directDiffuseBRDF * nDotL;
    result.specular += directLight.color * directSpecularBRDF * nDotL;
    return result;
  }
);

wgsl_fn!(
  // Reduces shadow mapping artifacts near tangent
  fn biasNDotL(nDotL: f32) -> f32 {
    return clamp(nDotL * 1.08 - 0.08, 0.0, 1.0);
  }
);

wgsl_fn!(
  // https://www.cs.cornell.edu/~srm/publications/EGSR07-btdf.pdf
  fn D_GGX(NoH: f32, roughness4: f32) -> f32 {
    let d = (NoH * roughness4 - NoH) * NoH + 1.0;
    // return roughness4 / (PI * d * d); todo support constant
    return roughness4 / (3.1415926 * d * d);
  }
);

wgsl_fn!(
  // NOTE: Basically same as
  // https://de45xmedrsdbp.cloudfront.net/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
  // However, calculate a F90 instead of using 1.0 directlly
  fn fresnel(vDotH: f32, f0: vec3<f32>) -> vec3<f32> {
    let fc = pow(1.0 - vDotH, 5.0);
    let f90 = clamp(f0 * 50.0, vec3<f32>(0.0), vec3<f32>(1.0));
    return f90 * fc + f0 * (1.0 - fc);
  }
);

wgsl_fn!(
  // NOTE: the microfacet model G part
  // TODO: Need reference for this part (1.0 or 0.5)
  fn visibility(nDotL: f32, nDotV: f32, roughness4: f32) -> f32 {
    let Vis_SmithV = nDotV + sqrt(nDotV * (nDotV - nDotV * roughness4) + roughness4);
    let Vis_SmithL = nDotL + sqrt(nDotL * (nDotL - nDotL * roughness4) + roughness4);
    return 1.0 / (Vis_SmithV * Vis_SmithL);
  }
);

wgsl_fn!(
  fn evaluateBRDFDiffuse(diffuseColor: vec3<f32>) -> vec3<f32> {
    // return INVERSE_PI * diffuseColor; todo support constant
    return 0.3183098 * diffuseColor;
  }
);

wgsl_fn!(
  fn evaluateBRDFSpecular(
    V: vec3<f32>,
    L: vec3<f32>,
    N: vec3<f32>,
    specularColor: vec3<f32>,
    roughness: f32,
  ) -> vec3<f32> {
    let EPSILON_SHADING = 0.0001; // todo constant
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
