use std::rc::Rc;

use rendiation_texture::TextureSampler;
use rendiation_webgpu::{
  BindGroupLayoutProvider, BindableResource, GPURenderPass, PipelineBuilder, WebGPUTextureCube, GPU,
};

use crate::*;

#[derive(Clone)]
pub struct EnvMapBackGroundMaterial {
  pub texture: SceneTextureCube,
  pub sampler: TextureSampler,
}

impl BindGroupLayoutProvider for EnvMapBackGroundMaterial {
  fn bind_preference() -> usize {
    1
  }
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: WebGPUTextureCube::bind_layout(),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::Sampler::bind_layout(),
          count: None,
        },
      ],
    })
  }

  fn gen_shader_header(group: usize) -> String {
    format!(
      "
      [[group({group}), binding(0)]]
      var r_color: texture_cube<f32>;

      [[group({group}), binding(1)]]
      var r_sampler: sampler;
    "
    )
  }

  fn register_uniform_struct_declare(_builder: &mut PipelineBuilder) {}
}

impl BackGroundShading for EnvMapBackGroundMaterial {
  fn shading(&self) -> &'static str {
    "
    fn background_shading(direction: vec3<f32>) -> vec3<f32> {
      return textureSample(r_color, r_sampler, direction).rgb;
    }
    "
  }
}

pub struct EnvMapBackGroundMaterialGPU {
  bindgroup: MaterialBindGroup,
}

impl MaterialGPUResource for EnvMapBackGroundMaterialGPU {
  type Source = EnvMapBackGroundMaterial;
  fn setup_pass_bindgroup<'a>(&self, pass: &mut GPURenderPass, ctx: &SceneMaterialPassSetupCtx) {
    pass.set_bind_group_owned(0, &ctx.model_gpu.unwrap().bindgroup, &[]);
    pass.set_bind_group_owned(1, &self.bindgroup.gpu, &[]);
    pass.set_bind_group_owned(2, &ctx.camera_gpu.bindgroup, &[]);
  }

  fn create_pipeline(
    &self,
    source: &Self::Source,
    builder: &mut PipelineBuilder,
    device: &wgpu::Device,
    ctx: &PipelineCreateCtx,
  ) {
    source.create_pipeline(builder, device, ctx)
  }
}

impl MaterialCPUResource for EnvMapBackGroundMaterial {
  type GPU = EnvMapBackGroundMaterialGPU;

  fn create(
    &self,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
    bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> Self::GPU {
    let bindgroup_layout = Self::layout(&gpu.device);
    let sampler = ctx.map_sampler(self.sampler, &gpu.device);
    let bindgroup = MaterialBindGroupBuilder::new(gpu, ctx.resources, bgw.clone())
      .push_texture(&self.texture)
      .push(sampler.as_bindable())
      .build(&bindgroup_layout);

    EnvMapBackGroundMaterialGPU { bindgroup }
  }

  fn is_keep_mesh_shape(&self) -> bool {
    false
  }
  fn is_transparent(&self) -> bool {
    false
  }
}
