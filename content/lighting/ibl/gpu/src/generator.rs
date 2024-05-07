use crate::*;

struct BrdfLUTGenerator;
impl ShaderPassBuilder for BrdfLUTGenerator {}
impl ShaderHashProvider for BrdfLUTGenerator {
  shader_hash_type_id! {}
}
impl GraphicsShaderProvider for BrdfLUTGenerator {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, _| {
      let sample_count = val(32);
      let uv = builder.query::<FragmentUv>().unwrap();
      let result = integrate_brdf(uv.x(), uv.y(), sample_count);
      builder.store_fragment_out(0, (result, val(1.), val(1.)))
    })
  }
}

pub fn generate_brdf_lut(ctx: &mut FrameCtx, target: GPU2DTextureView) {
  pass("brdf lut generate")
    .with_color(target, load())
    .render_ctx(ctx)
    .by(BrdfLUTGenerator.draw_quad());
}

// pub struct PrefilteredCubeMapPair {
//   diffuse: GPUTextureCube,
//   specular: GPUTextureCube,
// }

// pub fn prefilter(cube: GPUTextureCube) -> PrefilteredCubeMapPair {
//   todo!()
// }
