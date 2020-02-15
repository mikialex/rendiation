use crate::geometry::StandardGeometry;
use rendiation::*;

pub struct CopierShading {
  pipeline: WGPUPipeline,
}

impl CopierShading {
  pub fn new(renderer: &WGPURenderer, target: &WGPUTexture) -> Self {
    let mut pipeline_builder = WGPUPipelineDescriptorBuilder::new();
    pipeline_builder
      .vertex_shader(include_str!("./copy.vert"))
      .frag_shader(include_str!("./copy.frag"))
      .binding_group(
        BindGroupLayoutBuilder::new()
          .binding(wgpu::BindGroupLayoutBinding {
            binding: 0,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::SampledTexture {
              multisampled: false,
              dimension: wgpu::TextureViewDimension::D2,
            },
          })
          .binding(wgpu::BindGroupLayoutBinding {
            binding: 1,
            visibility: wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::Sampler,
          }),
      )
      .with_color_target(target);

    let pipeline = pipeline_builder.build::<StandardGeometry>(&renderer.device);

    Self { pipeline }
  }

  pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
    &self.pipeline.bind_group_layouts[0]
  }

  pub fn provide_pipeline(&self, pass: &mut WGPURenderPass, param: &CopyShadingParamGroup) {
    pass.gpu_pass.set_pipeline(&self.pipeline.pipeline);
    pass
      .gpu_pass
      .set_bind_group(0, &param.bindgroup.gpu_bindgroup, &[]);
  }
}

pub struct CopyShadingParamGroup {
  pub bindgroup: WGPUBindGroup,
}

impl CopyShadingParamGroup {
  pub fn new(
    renderer: &WGPURenderer,
    shading: &CopierShading,
    texture_view: &wgpu::TextureView,
    sampler: &WGPUSampler,
  ) -> Self {
    Self {
      bindgroup: BindGroupBuilder::new()
        .texture(texture_view)
        .sampler(sampler)
        .build(&renderer.device, shading.get_bind_group_layout()),
    }
  }
}
