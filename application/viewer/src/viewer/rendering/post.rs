use rendiation_texture_gpu_base::*;
use rendiation_texture_gpu_process::*;

use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default, PartialEq)]
pub struct PostEffects {
  pub enable_vignette: Bool,
  pub vignette: VignetteEffect,
  pub enable_chromatic_aberration: Bool,
  pub chromatic_aberration: ChromaticAberration,
}

pub struct PostProcess<'a, T> {
  pub input: AttachmentView<T>,
  pub config: &'a UniformBufferCachedDataView<PostEffects>,
}

impl<T> ShaderPassBuilder for PostProcess<'_, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.input);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    ctx.binding.bind(self.config);
  }
}

impl<T> ShaderHashProvider for PostProcess<'_, T> {
  shader_hash_type_id! {PostProcess< 'static, ()>}
}

impl<T> GraphicsShaderProvider for PostProcess<'_, T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let input_tex = binding.bind_by(&self.input);
      let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);
      let config = binding.bind_by(&self.config).load().expand();

      let uv = builder.query::<FragmentUv>();

      let input = config
        .enable_chromatic_aberration
        .into_bool()
        .select_branched(
          || chromatic_aberration_fn(uv, config.chromatic_aberration, input_tex, sampler),
          || input_tex.sample_zero_level(sampler, uv).xyz(),
        )
        .make_local_var();

      if_by(config.enable_vignette.into_bool(), || {
        input.store(compute_vignette_fn(uv, config.vignette, input.load()));
      });

      builder.store_fragment_out_vec4f(0, (input.load(), val(1.0)))
    });
  }
}

pub fn post_egui(ui: &mut egui::Ui, post: &UniformBufferCachedDataView<PostEffects>) {
  ui.collapsing("vignette", |ui| {
    post.mutate(|post| {
      let mut enabled: bool = post.enable_vignette.into();
      ui.checkbox(&mut enabled, "enabled");
      post.enable_vignette = enabled.into();

      ui.add(
        egui::Slider::new(&mut post.vignette.radius, 0.0..=1.0)
          .step_by(0.05)
          .text("radius"),
      );
      ui.add(
        egui::Slider::new(&mut post.vignette.feather, 0.0..=1.0)
          .step_by(0.05)
          .text("feather"),
      );
      ui.add(
        egui::Slider::new(&mut post.vignette.mid_point, 0.0..=1.0)
          .step_by(0.05)
          .text("mid_point"),
      );
    });
  });

  post.mutate(|post| {
    let mut enabled: bool = post.enable_chromatic_aberration.into();
    ui.checkbox(&mut enabled, "enable_chromatic_aberration");
    post.enable_chromatic_aberration = enabled.into();
  });
}
