use rendiation::*;

pub struct BlockShading {
  pipeline: WGPUPipeline,
}

struct PipelineStateBuilder{

}

impl BlockShading {
  pub fn new(renderer: &WGPURenderer, depth_target: &WGPUTexture) -> Self {
    let mut pipeline_builder = StaticPipelineBuilder::new(
      renderer,
      include_str!("./block.vert"),
      include_str!("./block.frag")
    );
    let pipeline = pipeline_builder
    .binding_group::<BlockShadingParamGroup>()
    .geometry::<StandardGeometry>()
    .to_screen_target()
    .with_depth_stencil(depth_target)
    .build();
    Self { pipeline }
  }

  pub fn provide_pipeline(&self, pass: &mut WGPURenderPass, bg: &WGPUBindGroup) {
    pass.gpu_pass.set_pipeline(&self.pipeline.pipeline);
    pass
      .gpu_pass
      .set_bind_group(0, &bg.gpu_bindgroup, &[]);
  }
}

use rendiation_marco::BindGroup;

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

pub struct BlockShadingParamGroupGPU(WGPUBindGroup);