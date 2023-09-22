use crate::*;

mod cube;
mod d2;
mod mipmap_gen;
mod pair;
mod sampler;

pub use cube::*;
pub use d2::*;
pub use mipmap_gen::*;
pub use pair::*;
pub use sampler::*;

/// note: we could design beautiful apis like Stream<Item = GPU2DTextureView> -> Stream<Item =
/// Texture2DHandle>, but for now, we require bindless downgrade ability, so we directly combined
/// the handle with the resource in BindableGPUChange
#[derive(Clone)]
pub enum BindableGPUChange {
  Reference2D(GPU2DTextureView, Texture2DHandle),
  ReferenceCube(GPUCubeTextureView),
  ReferenceSampler(GPUSamplerView, SamplerHandle),
  Content,
}

impl BindableGPUChange {
  fn into_render_component_delta(self) -> RenderComponentDeltaFlag {
    match self {
      BindableGPUChange::Reference2D(..) => RenderComponentDeltaFlag::ContentRef,
      BindableGPUChange::ReferenceCube(..) => RenderComponentDeltaFlag::ContentRef,
      BindableGPUChange::ReferenceSampler(..) => RenderComponentDeltaFlag::ContentRef,
      BindableGPUChange::Content => RenderComponentDeltaFlag::Content,
    }
  }
}

struct BrdfLUTGenerator;
impl ShaderPassBuilder for BrdfLUTGenerator {}
impl ShaderHashProvider for BrdfLUTGenerator {}
impl GraphicsShaderProvider for BrdfLUTGenerator {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, _| {
      let sample_count = val(32);
      let uv = builder.query::<FragmentUv>().unwrap();
      let result = rendiation_lighting_ibl_core::integrate_brdf(uv.x(), uv.y(), sample_count);
      builder.store_fragment_out(0, (result, val(1.), val(1.)))
    })
  }
}

pub fn generate_brdf_lut(
  ctx: &mut FrameCtx,
  target: GPU2DTextureView,
  generator: &dyn RenderComponentAny,
) {
  pass("brdf lut generate")
    .with_color(target, load())
    .render(ctx)
    .by(generator.draw_quad());
}
