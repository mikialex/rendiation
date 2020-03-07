use rendiation::*;
use rendiation_marco::BindGroup;

pub struct QuadShading {
  pub pipeline: WGPUPipeline,
}

#[derive(BindGroup)]
pub struct QuadShadingParam<'a> {
  #[bind_type = "uniform-buffer:vertex"]
  pub buffer: &'a WGPUBuffer,
}

impl QuadShading {
  pub fn new(renderer: &WGPURenderer, target: &WGPUTexture) -> Self {
    let mut pipeline_builder = StaticPipelineBuilder::new(
      renderer,
      include_str!("./quad.vert"),
      include_str!("./quad.frag"),
    );
    let pipeline = pipeline_builder
      .geometry::<StandardGeometry>()
      .binding_group::<QuadShadingParam>()
      .to_color_target(target)
      .build();
    Self { pipeline }
  }
}

pub struct CopyShading {
  pub pipeline: WGPUPipeline,
}

impl CopyShading {
  pub fn new(renderer: &WGPURenderer) -> Self {
    let mut pipeline_builder = StaticPipelineBuilder::new(
      renderer,
      include_str!("./copy.vert"),
      include_str!("./copy.frag"),
    );
    let pipeline = pipeline_builder
      .geometry::<StandardGeometry>()
      .binding_group::<CopyShadingParam>()
      .color_blend(wgpu::BlendDescriptor {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
      })
      .to_screen_target()
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
