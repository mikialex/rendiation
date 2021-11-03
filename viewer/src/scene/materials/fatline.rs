use rendiation_webgpu::*;
use std::rc::Rc;

use crate::*;

#[derive(Clone)]
pub struct FatLineMaterial {
  pub width: f32,
  pub states: MaterialStates,
}

pub struct FatlineMaterialGPU {
  _uniform: UniformBuffer<f32>,
  bindgroup: MaterialBindGroup,
}

impl BindGroupLayoutProvider for FatLineMaterial {
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: UniformBuffer::<f32>::bind_layout(),
        count: None,
      }],
    })
  }
}

impl FatLineMaterial {
  pub fn get_shader_header() -> &'static str {
    "
    [[block]]
    struct FatlineMaterial {
      width: f32;
    };

    [[group(1), binding(0)]]
    var<uniform> fatline_material: FatlineMaterial;
    "
  }
}

impl PipelineRequester for FatlineMaterialGPU {
  type Container = PipelineUnit;
}

impl MaterialGPUResource for FatlineMaterialGPU {
  type Source = FatLineMaterial;

  fn pipeline_key(
    &self,
    _source: &Self::Source,
    _ctx: &PipelineCreateCtx,
  ) -> <Self::Container as PipelineVariantContainer>::Key {
  }
  fn create_pipeline(
    &self,
    _source: &Self::Source,
    builder: &mut PipelineBuilder,
    device: &wgpu::Device,
    ctx: &PipelineCreateCtx,
  ) {
    builder.shader_source = format!(
      "
      {object_header}
      {material_header}
      {camera_header}

      struct VertexOutput {{
        [[builtin(position)]] position: vec4<f32>;
        [[location(0)]] uv: vec2<f32>;
      }};

      [[stage(vertex)]]
      fn vs_main(
        {vertex_header}
      ) -> VertexOutput {{
        var out: VertexOutput;
        out.uv = uv;
        out.position = camera.projection * camera.view * model.matrix * vec4<f32>(position, 1.0);
        return out;
      }}
      
      [[stage(fragment)]]
      fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
          return textureSample(r_color, r_sampler, in.uv);
      }}
      
      ",
      vertex_header = FatLineVertex::get_shader_header(),
      material_header = FatLineMaterial::get_shader_header(),
      camera_header = CameraBindgroup::get_shader_header(),
      object_header = TransformGPU::get_shader_header(),
    );

    builder.with_layout(ctx.layouts.retrieve::<FatLineMaterial>(device));

    builder.vertex_buffers = ctx.active_mesh.unwrap().vertex_layout();
  }

  fn setup_pass_bindgroup<'a>(
    &self,
    pass: &mut GPURenderPass<'a>,
    _ctx: &SceneMaterialPassSetupCtx,
  ) {
    pass.set_bind_group_owned(1, &self.bindgroup.gpu, &[]);
  }
}

impl MaterialCPUResource for FatLineMaterial {
  type GPU = FatlineMaterialGPU;

  fn create(
    &mut self,
    gpu: &GPU,
    _ctx: &mut SceneMaterialRenderPrepareCtx,
    bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> Self::GPU {
    let device = &gpu.device;
    let _uniform = UniformBuffer::create(device, self.width);

    let bindgroup_layout = Self::layout(device);
    let bindgroup = MaterialBindGroupBuilder::new(gpu, bgw.clone())
      .push(_uniform.gpu().as_entire_binding())
      .build(&bindgroup_layout);

    FatlineMaterialGPU {
      _uniform,
      bindgroup,
    }
  }
}
