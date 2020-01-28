use crate::geometry::StandardGeometry;
use rendiation::*;

pub struct TestShading {
  pipeline: WGPUPipeline,
  // bindgroup: Option<WGPUBindGroup>,

  // texture: (usize, usize),
  // matrix_uniform_buffer: (usize, usize),
}

impl TestShading {
  pub fn new(renderer: &WGPURenderer) -> Self {
    let mut pipeline_builder = WGPUPipelineDescriptorBuilder::new();
    pipeline_builder
      .vertex_shader(include_str!("./shader/test.vert"))
      .frag_shader(include_str!("./shader/test.frag"))
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
      );

    let pipeline = pipeline_builder
      .build::<StandardGeometry>(&renderer.device, &renderer.swap_chain.swap_chain_descriptor);

    Self { pipeline }
  }

  pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout{
    &self.pipeline.bind_group_layouts[0]
  }

  pub fn provide_pipeline(&self, pass: &mut WGPURenderPass, param: &TestShadingParamGroup) {
    pass.gpu_pass.set_pipeline(&self.pipeline.pipeline);
    pass.gpu_pass.set_bind_group(0, &param.bindgroup.gpu_bindgroup, &[]);
  }
}

pub struct TestShadingParamGroup{
  pub bindgroup: WGPUBindGroup
}

impl TestShadingParamGroup{
  pub fn new(
    renderer: &WGPURenderer,
    shading: &TestShading,
    texture_view: &wgpu::TextureView,
    sampler: &WGPUSampler,
    buffer: &WGPUBuffer,
  ) -> Self {
    TestShadingParamGroup{
      bindgroup: 
      BindGroupBuilder::new()
        .buffer(buffer)
        .texture(texture_view)
        .sampler(sampler)
        .build(&renderer.device, shading.get_bind_group_layout())
    }
  }
}