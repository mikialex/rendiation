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
          .bind_texture2d(ShaderStage::Fragment)
          .bind_sampler(ShaderStage::Fragment),
      )
      .with_color_target(target);

    let pipeline = pipeline_builder.build::<StandardGeometry>(&renderer.device);

    Self { pipeline }
  }

  pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
    &self.pipeline.get_bindgroup_layout(0)
  }

  pub fn provide_pipeline(&self, pass: &mut WGPURenderPass, param: &CopyShadingParamGroup) {
    pass.gpu_pass.set_pipeline(&self.pipeline.pipeline);
    pass
      .gpu_pass
      .set_bind_group(0, &param.bindgroup.gpu_bindgroup, &[]);
  }
}

// struct CopyParam<'a> {
//   pub texture: &'a wgpu::TextureView,
//   pub sampler: &'a WGPUSampler,
//   pub bindgroup: Option<WGPUBindGroup>,
// }

// pub trait BindGroupProvider {
//   fn provide_layout() -> BindGroupLayoutBuilder;
//   fn create_bindgroup(&mut self, renderer: &mut WGPURenderer) -> WGPUBindGroup;
// }

// impl<'a> BindGroupProvider for CopyParam<'a> {
//   fn provide_layout() -> BindGroupLayoutBuilder {
//     BindGroupLayoutBuilder::new()
//       .bind_texture2d(ShaderStage::Fragment)
//       .bind_sampler(ShaderStage::Fragment)
//   }

//   fn create_bindgroup(&mut self, renderer: &mut WGPURenderer) -> WGPUBindGroup {
//     BindGroupBuilder::new()
//       .texture(self.texture)
//       .sampler(self.sampler)
//       .build(&renderer.device, shading.get_bind_group_layout())
//   }
// }

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
