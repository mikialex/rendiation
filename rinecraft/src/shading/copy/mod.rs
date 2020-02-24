use crate::geometry::StandardGeometry;
use rendiation::*;

pub struct CopierShading {
  pipeline: WGPUPipeline,
}

impl CopierShading {
  pub fn new(renderer: &WGPURenderer, target: &WGPUTexture) -> Self {
    let mut pipeline_builder = StaticPipelineBuilder::new(
      &renderer,
      include_str!("./copy.vert"),
      include_str!("./copy.frag"),
    );
    let pipeline = pipeline_builder
      .binding_group::<CopyParam>()
      .geometry::<StandardGeometry>()
      .to_color_target(target)
      .build();

    Self {
      pipeline,
    }
  }

  pub fn provide_pipeline(&self, pass: &mut WGPURenderPass, param: &CopyParamGPU) {
    pass.gpu_pass.set_pipeline(&self.pipeline.pipeline);
    pass
      .gpu_pass
      .set_bind_group(0, &param.bindgroup.gpu_bindgroup, &[]);
  }
}

struct CopyParam<'a> {
  pub texture: &'a wgpu::TextureView,
  pub sampler: &'a WGPUSampler,
  pub bindgroup: Option<WGPUBindGroup>,
}

static mut COPY_PARAM_LAYOUT: Option<wgpu::BindGroupLayout> = None;

impl<'a> BindGroupProvider for CopyParam<'a> {
  fn provide_layout(renderer: &WGPURenderer) -> &'static wgpu::BindGroupLayout {
    unsafe {
      if let Some(layout) = &COPY_PARAM_LAYOUT {
        &layout
      } else {
        let builder = BindGroupLayoutBuilder::new()
          .bind_texture2d(ShaderStage::Fragment)
          .bind_sampler(ShaderStage::Fragment);
        let layout = renderer
          .device
          .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &builder.bindings,
          });
        COPY_PARAM_LAYOUT = Some(layout);
        COPY_PARAM_LAYOUT.as_ref().unwrap()
      }
    }
  }

  fn create_bindgroup(&mut self, renderer: &WGPURenderer) -> WGPUBindGroup {
    BindGroupBuilder::new()
      .texture(self.texture)
      .sampler(self.sampler)
      .build(&renderer.device, CopyParam::provide_layout(renderer))
  }
}

pub struct CopyParamGPU {
  pub bindgroup: WGPUBindGroup,
}

impl CopyParamGPU {
  pub fn new(
    renderer: &WGPURenderer,
    param: &CopyParam,
  ) -> Self {
    Self {
      bindgroup: param.create_bindgroup(renderer)
    }
  }
}
