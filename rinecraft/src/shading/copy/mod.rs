use rendiation_webgpu::*;
use rendiation_mesh_buffer::geometry::*;

pub struct CopierShading {
  pub pipeline: WGPUPipeline,
}

impl CopierShading {
  pub fn new(renderer: &WGPURenderer, target: &RenderTarget) -> Self {
    let pipeline = PipelineBuilder::new(
      &renderer,
      load_glsl(include_str!("./copy.vert"), ShaderType::Vertex),
      load_glsl(include_str!("./copy.frag"), ShaderType::Fragment),
    )
    .as_mut()
    .binding_group::<CopyParam>()
    .geometry::<IndexedGeometry>()
    .target_states(target.create_target_states().as_ref())
    .build();

    Self { pipeline }
  }
}

use render_target::{RenderTarget, TargetStatesProvider};
use rendiation_webgpu_derives::BindGroup;

#[derive(BindGroup)]
pub struct CopyParam<'a> {
  #[bind_type = "texture2d:fragment"]
  pub texture: &'a wgpu::TextureView,

  #[bind_type = "sampler:fragment"]
  pub sampler: &'a WGPUSampler,
}
