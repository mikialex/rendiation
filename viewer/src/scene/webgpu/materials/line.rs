use std::rc::Rc;

use bytemuck::*;
use rendiation_algebra::*;
use rendiation_webgpu::*;

use crate::*;

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct LineMaterial {
  pub color: Vec4<f32>,
}

impl ShaderUniformBlock for LineMaterial {
  fn shader_struct() -> &'static str {
    "
    [[block]]
    struct LineMaterial {
      color: vec4<f32>;
    };
    "
  }
}

impl BindGroupLayoutProvider for LineMaterial {
  fn bind_preference() -> usize {
    1
  }
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: UniformBuffer::<LineMaterial>::bind_layout(),
        count: None,
      }],
    })
  }

  fn gen_shader_header(group: usize) -> String {
    format!(
      "
      [[group({group}), binding(0)]]
      var<uniform> line_material: FlatMaterial;
    
    ",
    )
  }

  fn register_uniform_struct_declare(builder: &mut PipelineBuilder) {
    builder.declare_uniform_struct::<LineMaterial>();
  }
}

pub struct LineMaterialGPU {
  _uniform: UniformBuffer<LineMaterial>,
  bindgroup: MaterialBindGroup,
}

impl MaterialGPUResource for LineMaterialGPU {
  type Source = LineMaterial;

  fn create_pipeline(
    &self,
    _source: &Self::Source,
    builder: &mut PipelineBuilder,
    device: &wgpu::Device,
    ctx: &PipelineCreateCtx,
  ) {
    builder
      .include_fragment_entry(
        "
      [[stage(fragment)]]
      fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
          return line_material.color;
      }}
      ",
      )
      .use_fragment_entry("fs_main");

    builder.with_layout::<LineMaterial>(ctx.layouts, device);

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

impl MaterialCPUResource for LineMaterial {
  type GPU = LineMaterialGPU;

  fn create(
    &mut self,
    gpu: &GPU,
    _ctx: &mut SceneMaterialRenderPrepareCtx,
    bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> Self::GPU {
    let _uniform = UniformBuffer::create(&gpu.device, *self);

    let bindgroup_layout = Self::layout(&gpu.device);

    let bindgroup = MaterialBindGroupBuilder::new(gpu, bgw.clone())
      .push(_uniform.as_bindable())
      .build(&bindgroup_layout);

    LineMaterialGPU {
      _uniform,
      bindgroup,
    }
  }

  fn is_keep_mesh_shape(&self) -> bool {
    true
  }

  fn is_transparent(&self) -> bool {
    false
  }
}

#[derive(Clone)]
pub struct LineDash {
  pub screen_spaced: bool,
  pub scale: f32,
  pub gap_size: f32,
  pub dash_size: f32,
  pub view_scale: f32,
}
