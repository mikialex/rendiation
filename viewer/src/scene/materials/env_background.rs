use rendiation_texture::TextureSampler;
use rendiation_webgpu::{BindableResource, WebGPUTextureCube, GPU};

use crate::{
  BackGroundShading, MaterialBindGroup, MaterialCPUResource, MaterialGPUResource, MaterialHandle,
  PipelineUnit, PipelineVariantContainer, SceneMaterialPassSetupCtx, SceneMaterialRenderPrepareCtx,
  TextureCubeHandle, ViewerDeviceExt,
};

#[derive(Clone)]
pub struct EnvMapBackGroundMaterial {
  pub texture: TextureCubeHandle,
  pub sampler: TextureSampler,
}

impl BackGroundShading for EnvMapBackGroundMaterial {
  fn shading(&self) -> &'static str {
    "
    fn background_shading(direction: vec3<f32>) -> vec3<f32> {
      textureSample(r_color, r_sampler, direction);
    }
    "
  }

  fn shader_header(&self) -> &'static str {
    "
    [[group(1), binding(0)]]
    var r_color: texture_cube<f32>;

    [[group(1), binding(1)]]
    var r_sampler: sampler;
    "
  }

  fn create_bindgroup_layout(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
}

pub struct EnvMapBackGroundMaterialGPU {
  bindgroup: MaterialBindGroup,
}

impl MaterialGPUResource for EnvMapBackGroundMaterialGPU {
  type Source = EnvMapBackGroundMaterial;
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a>,
  ) {
    let pipeline = ctx
      .pipelines
      .get_cache::<Self, PipelineUnit>()
      .retrieve(&());

    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &ctx.model_gpu.bindgroup, &[]);
    pass.set_bind_group(1, &self.bindgroup.gpu, &[]);
    pass.set_bind_group(2, &ctx.camera_gpu.bindgroup, &[]);
  }

  fn request_pipeline(
    &mut self,
    source: &Self::Source,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) {
    let (pipelines, pipeline_ctx) = ctx.pipeline_ctx();
    pipelines
      .get_cache_mut::<Self, PipelineUnit>()
      .request(&(), || source.create_pipeline(&gpu.device, &pipeline_ctx));
  }
}

impl MaterialCPUResource for EnvMapBackGroundMaterial {
  type GPU = EnvMapBackGroundMaterialGPU;

  fn create(
    &mut self,
    handle: MaterialHandle,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) -> Self::GPU {
    let bindgroup_layout = self.create_bindgroup_layout(&gpu.device);
    let sampler = ctx.map_sampler(self.sampler, &gpu.device);
    let bindgroup = gpu
      .device
      .material_bindgroup_builder(handle)
      .push_texture_cube(ctx, self.texture)
      .push(sampler.as_bindable())
      .build(&bindgroup_layout);

    EnvMapBackGroundMaterialGPU { bindgroup }
  }
}
