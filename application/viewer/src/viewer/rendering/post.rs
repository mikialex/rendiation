use rendiation_texture_gpu_process::*;

use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct PostEffects {
  pub vignette: VignetteEffect,
  pub chromatic_aberration: ChromaticAberration,
}

pub struct PostProcess<'a, T> {
  pub input: AttachmentView<T>,
  pub config: &'a UniformBufferCachedDataView<PostEffects>,
}

impl<'a, T> ShaderPassBuilder for PostProcess<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.input);
  }
}

impl<'a, T> ShaderHashProvider for PostProcess<'a, T> {
  shader_hash_type_id! {PostProcess< 'static, ()>}
}

impl<'a, T> GraphicsShaderProvider for PostProcess<'a, T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      //   let highlighter = binding.bind_by(&self.lighter.data).load().expand();

      //   let mask = binding.bind_by(&self.input);
      //   let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

      //   let uv = builder.query::<FragmentUv>();
      //   let size = builder.query::<RenderBufferSize>();

      //   builder.store_fragment_out(
      //     0,
      //     (
      //       highlighter.color.xyz(),
      //       edge_intensity_fn(uv, mask, sampler, highlighter.width, size) * highlighter.color.w(),
      //     ),
      //   )
    })
  }
}
