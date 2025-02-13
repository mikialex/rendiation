use rendiation_algebra::*;
use rendiation_lighting_gpu_system::*;
use rendiation_lighting_transport::*;
use rendiation_shader_api::*;
use rendiation_texture_core::Texture2DBuffer;

mod brdf;
mod fit;
mod shader;

pub use brdf::*;
pub use fit::*;
pub use shader::*;

/// This const is a check flag to keep the shader impl sync with lut content generation.
/// User could use this version as their lut cache suffix to determine if the lut cache is invalid.
/// If any REAL lut generation logic changed, we will update this version.
///
/// 0: the original ltc impl
pub const LTC_LUT_VERSION: u64 = 0;

pub struct LTCRectLightingCompute {
  pub light: Node<LTCRectLight>,
  pub lut: LTCxLUTxInvocation,
}

#[derive(Clone, Copy)]
pub struct LTCxLUTxInvocation {
  pub ltc_1: HandleNode<ShaderTexture2D>,
  pub ltc_2: HandleNode<ShaderTexture2D>,
  pub sampler: HandleNode<ShaderSampler>,
}

impl LightingComputeInvocation for LTCRectLightingCompute {
  fn compute_lights(
    &self,
    shading: &dyn LightableSurfaceShading,
    geom_ctx: &ENode<ShaderLightingGeometricCtx>,
  ) -> ENode<ShaderLightingResult> {
    let shading = shading
      .as_any()
      .downcast_ref::<ENode<ShaderPhysicalShading>>();
    if shading.is_none() {
      return zeroed_val::<ShaderLightingResult>().expand();
    }
    let shading = shading.unwrap();

    LTCxLightEval {
      light: self.light,
      diffuse_color: shading.diffuse,
      specular_color: shading.f0,              // todo fix
      roughness: shading.perceptual_roughness, // todo check
      geom: *geom_ctx,
      lut: self.lut,
    }
    .eval()
  }
}
