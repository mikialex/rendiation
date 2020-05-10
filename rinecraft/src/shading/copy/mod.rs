use rendiation::*;

pub struct CopierShading {
  pipeline: WGPUPipeline,
}

impl CopierShading {
  pub fn new(renderer: &WGPURenderer, target: &RenderTarget) -> Self {
    let pipeline = StaticPipelineBuilder::new(
      &renderer,
      include_str!("./copy.vert"),
      include_str!("./copy.frag"),
    )
    .as_mut()
    .binding_group::<CopyParam>()
    .geometry::<StandardGeometry>()
    .target_states(target.create_target_states().as_ref())
    .build();

    Self { pipeline }
  }

  pub fn provide_pipeline(&self, pass: &mut WGPURenderPass, param: &WGPUBindGroup) {
    pass.gpu_pass.set_pipeline(&self.pipeline.pipeline);
    pass.gpu_pass.set_bind_group(0, &param.gpu_bindgroup, &[]);
  }
}

use rendiation_marco::BindGroup;
use render_target::RenderTarget;

#[derive(BindGroup)]
pub struct CopyParam<'a> {
  #[bind_type = "texture2d:fragment"]
  pub texture: &'a wgpu::TextureView,

  #[bind_type = "sampler:fragment"]
  pub sampler: &'a WGPUSampler,
}
