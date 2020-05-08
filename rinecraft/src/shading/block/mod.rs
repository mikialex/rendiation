use rendiation::*;

use rendiation_marco::BindGroup;
use scene::resource::Shading;

pub fn create_block_shading(renderer: &WGPURenderer) -> Shading {
  let mut pipeline_builder = StaticPipelineBuilder::new(
    renderer,
    include_str!("./block.vert"),
    include_str!("./block.frag"),
  );
  let pipeline = pipeline_builder
    .binding_group::<BlockShadingParamGroup>()
    .geometry::<StandardGeometry>()
    .to_screen_target()
    .with_default_depth()
    .build();
  Shading::new(pipeline)
}

#[derive(BindGroup)]
pub struct BlockShadingParamGroup<'a> {
  #[bind_type = "uniform-buffer:vertex"]
  pub u_mvp_matrix: &'a WGPUBuffer,

  #[bind_type = "texture2d:fragment"]
  pub texture_view: &'a wgpu::TextureView,

  #[bind_type = "sampler:fragment"]
  pub sampler: &'a WGPUSampler,

  #[bind_type = "uniform-buffer:fragment"]
  pub u_camera_world_position: &'a WGPUBuffer,
}
