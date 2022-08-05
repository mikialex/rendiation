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
    todo!()
  }
}

// // wgsl_function!(
//   fn physical_shading(
//     directLight: ShaderIncidentLight,
//     geometry: ShaderLightingGeometricCtx,
//     shading: PhysicalShading,
//   ) -> ShaderLightingResult {
//      let nDotL = biasNDotL(dot(-directLight.direction, geometry.normal));
//     if (nDotL == 0.0)
//       return;
//     vec3 directDiffuseBRDF = evaluateBRDFDiffuse(material.diffuse);
//     vec3 directSpecularBRDF = evaluateBRDFSpecular(
//       geometry.viewDir, -directLight.direction,
//     geometry.normal, material.specular, material.roughness);
//     reflectedLight.directDiffuse += directLight.color * directDiffuseBRDF * nDotL;
//     reflectedLight.directSpecular += directLight.color * directSpecularBRDF * nDotL;
//   }
// // );

// // Reduces shadow mapping artifacts near tangent
// float biasNDotL(const in float nDotL) {
//   return clamp(nDotL * 1.08 - 0.08, 0.0, 1.0);
// }

// // https://www.cs.cornell.edu/~srm/publications/EGSR07-btdf.pdf
// float D_GGX(const float NoH, const float roughness4) {
// 	float d = ( NoH * roughness4 - NoH ) * NoH + 1.0;
// 	return roughness4 / ( PI * d * d );
// }

// // NOTE: Basically same as
// // https://de45xmedrsdbp.cloudfront.net/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
// // However, calculate a F90 instead of using 1.0 directlly
// float fresnel(const in float vDotH, const in float f0) {
//   float fc = pow(1.0 - vDotH, 5.0);
//   float f90 = clamp(f0 * 50.0, 0.0, 1.0);
//   return f90 * fc + f0 * (1.0 - fc);
// }

// // NOTE: the microfacet model G part
// // TODO: Need reference for this part (1.0 or 0.5)
// float visibility(const in float nDotL, const in float nDotV, const in float roughness4) {
// 	float Vis_SmithV = nDotV + sqrt( nDotV * (nDotV - nDotV * roughness4) + roughness4 );
// 	float Vis_SmithL = nDotL + sqrt( nDotL * (nDotL - nDotL * roughness4) + roughness4 );
// 	return 1.0 / ( Vis_SmithV * Vis_SmithL );
// }

// vec3 evaluateBRDFDiffuse(const in vec3 diffuseColor)
// {
//     return INVERSE_PI * diffuseColor;
// }

// vec3 evaluateBRDFSpecular(
//     const in vec3 V,
//     const in vec3 L,
//     const in vec3 N,
//     const in vec3 specularColor,
//     const in float roughness)
// {
//     vec3 H = normalize(L + V);
//     float nDotL = max(dot(L, N), 0.0);
//     float nDotV = max(EPSILON_SHADING, dot(N, V));
//     float nDotH = max(EPSILON_SHADING, dot(N, H));
//     float vDotH = max(EPSILON_SHADING, dot(V, H));
//     float roughness2 = roughness * roughness;
//     float roughness4 = roughness2 * roughness2;

//     vec3 f = fresnel(vDotH, specularColor);
//     float d = max(D_GGX(nDotH, roughness4), 0.0);
//     float g = max(visibility(nDotL, nDotV, roughness4), 0.0);

//     return f * (d * g);
// }
