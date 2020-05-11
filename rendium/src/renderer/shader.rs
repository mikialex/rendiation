use render_target::{TargetStatesProvider, RenderTarget};
use rendiation::*;
use rendiation_marco::BindGroup;

pub struct QuadShading {
  pub pipeline: WGPUPipeline,
}

#[derive(BindGroup)]
pub struct QuadShadingParam<'a> {
  #[bind_type = "uniform-buffer:vertex"]
  pub transform: &'a WGPUBuffer,

  #[bind_type = "uniform-buffer:fragment"]
  pub color: &'a WGPUBuffer,
}

impl QuadShading {
  pub fn new(renderer: &WGPURenderer, target: &RenderTarget) -> Self {
    let pipeline = PipelineBuilder::new(
      renderer,
      include_str!("./quad.vert"),
      include_str!("./quad.frag"),
    )
    .as_mut()
    .geometry::<StandardGeometry>()
    .binding_group::<QuadShadingParam>()
    .target_states(target.create_target_states().as_ref())
    .build();
    Self { pipeline }
  }
}

pub struct CopyShading {
  pub pipeline: WGPUPipeline,
}

impl CopyShading {
  pub fn new(renderer: &WGPURenderer, target: & impl TargetStatesProvider) -> Self {
    let pipeline = PipelineBuilder::new(
      renderer,
      include_str!("./copy.vert"),
      include_str!("./copy.frag"),
    )
    .as_mut()
    .geometry::<StandardGeometry>()
    .binding_group::<CopyShadingParam>()
    .target_states(target.create_target_states().as_mut().first_color(|s| {
      s.color_blend(wgpu::BlendDescriptor {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
      })
    }))
    .build();
    Self { pipeline }
  }
}

#[derive(BindGroup)]
pub struct CopyShadingParam<'a> {
  #[bind_type = "texture2d:fragment"]
  pub texture_view: &'a wgpu::TextureView,

  #[bind_type = "sampler:fragment"]
  pub sampler: &'a WGPUSampler,
}
