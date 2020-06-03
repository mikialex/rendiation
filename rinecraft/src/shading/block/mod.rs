use rendiation::*;
use rendiation_mesh_buffer::geometry::*;

use render_target::TargetStates;
use rendiation_derives::BindGroup;

pub fn create_block_shading(renderer: &WGPURenderer, target: &TargetStates) -> WGPUPipeline {
  PipelineBuilder::new(
    renderer,
    load_glsl(include_str!("./block.vert"), ShaderType::Vertex),
    load_glsl(include_str!("./block.frag"), ShaderType::Fragment),
  )
  .as_mut()
  .binding_group::<BlockShadingParamGroup>()
  .geometry::<IndexedGeometry>()
  .target_states(target)
  .build()
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
