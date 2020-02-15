use crate::geometry::StandardGeometry;
use rendiation::*;

pub struct BlockShading {
  pipeline: WGPUPipeline,
}

impl BlockShading {
  pub fn new(renderer: &WGPURenderer) -> Self {
    let mut pipeline_builder = WGPUPipelineDescriptorBuilder::new();
    pipeline_builder
      .vertex_shader(include_str!("./block.vert"))
      .frag_shader(include_str!("./block.frag"))
      .binding_group(
        BindGroupLayoutBuilder::new()
          .binding(wgpu::BindGroupLayoutBinding {
            binding: 0,
            visibility: wgpu::ShaderStage::VERTEX,
            ty: wgpu::BindingType::UniformBuffer { dynamic: false },
          })
          .binding(wgpu::BindGroupLayoutBinding {
            binding: 1,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::SampledTexture {
              multisampled: false,
              dimension: wgpu::TextureViewDimension::D2,
            },
          })
          .binding(wgpu::BindGroupLayoutBinding {
            binding: 2,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::Sampler,
          }),
      )
      .with_swapchain_target(&renderer.swap_chain.swap_chain_descriptor);

    let pipeline = pipeline_builder.build::<StandardGeometry>(&renderer.device);

    Self { pipeline }
  }

  pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
    &self.pipeline.bind_group_layouts[0]
  }

  pub fn provide_pipeline(&self, pass: &mut WGPURenderPass, param: &BlockShadingParamGroup) {
    pass.gpu_pass.set_pipeline(&self.pipeline.pipeline);
    pass
      .gpu_pass
      .set_bind_group(0, &param.bindgroup.gpu_bindgroup, &[]);
  }
}

pub struct BlockShadingParamGroup {
  pub bindgroup: WGPUBindGroup,
}

impl BlockShadingParamGroup {
  pub fn new(
    renderer: &WGPURenderer,
    shading: &BlockShading,
    texture_view: &wgpu::TextureView,
    sampler: &WGPUSampler,
    buffer: &WGPUBuffer,
  ) -> Self {
    Self {
      bindgroup: BindGroupBuilder::new()
        .buffer(buffer)
        .texture(texture_view)
        .sampler(sampler)
        .build(&renderer.device, shading.get_bind_group_layout()),
    }
  }
}
