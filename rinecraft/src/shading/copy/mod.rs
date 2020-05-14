use rendiation::*;

pub struct CopierShading {
  pub pipeline: WGPUPipeline,
}

impl CopierShading {
  pub fn new(renderer: &WGPURenderer, target: &RenderTarget) -> Self {
    let pipeline = PipelineBuilder::new(
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

}

use rendiation_marco::BindGroup;
use render_target::{RenderTarget, TargetStatesProvider};

#[derive(BindGroup)]
pub struct CopyParam<'a> {
  #[bind_type = "texture2d:fragment"]
  pub texture: &'a wgpu::TextureView,

  #[bind_type = "sampler:fragment"]
  pub sampler: &'a WGPUSampler,
}
