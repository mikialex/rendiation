use crate::geometry::StandardGeometry;
use rendiation::*;

pub struct BlockShading {
  pipeline: WGPUPipeline,
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

// use rendiation_marco::BindGroup;

// #[derive(BindGroup)]
// pub struct BlockShadingParamGroup<'a> {
  
//   #[bind_type = "texture2d-fragment"]
//   pub texture_view: &'a wgpu::TextureView,
  
//   #[bind_type = "sampler-fragment"]
//   pub sampler: &'a WGPUSampler,

//   #[bind_type = "uniform-buffer-vertex"]
//   pub buffer: &'a WGPUBuffer,
// }

pub struct BlockShadingParamGroup<'a> {
  
  pub texture_view: &'a wgpu::TextureView,
  
  pub sampler: &'a WGPUSampler,

  pub buffer: &'a WGPUBuffer,
}



static mut BLOCK_PARAM_LAYOUT: Option<wgpu::BindGroupLayout> = None;

impl<'a> BindGroupProvider for BlockShadingParamGroup<'a> {
  fn provide_layout(renderer: &WGPURenderer) -> &'static wgpu::BindGroupLayout {
    unsafe {
      if let Some(layout) = &BLOCK_PARAM_LAYOUT {
        &layout
      } else {
        let builder = BindGroupLayoutBuilder::new()
          .bind_texture2d(ShaderType::Fragment)
          .bind_sampler(ShaderType::Fragment)
          .bind_uniform_buffer(ShaderType::Vertex);
        let layout = renderer
          .device
          .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &builder.bindings,
          });
          BLOCK_PARAM_LAYOUT = Some(layout);
          BLOCK_PARAM_LAYOUT.as_ref().unwrap()
      }
    }
  }

  fn create_bindgroup(&self, renderer: &WGPURenderer) -> WGPUBindGroup {
    BindGroupBuilder::new()
      .texture(self.texture_view)
      .sampler(self.sampler)
      .buffer(self.buffer)
      .build(&renderer.device, BlockShadingParamGroup::provide_layout(renderer))
  }
}