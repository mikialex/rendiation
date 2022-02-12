use std::rc::Rc;

use rendiation_algebra::Vec4;
use rendiation_renderable_mesh::vertex::Vertex;
use rendiation_webgpu::*;

use crate::*;

impl MaterialMeshLayoutRequire for FlatMaterial {
  type VertexInput = Vec<Vertex>;
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderUniform)]
pub struct FlatMaterialUniform {
  pub color: Vec4<f32>,
}

impl SemanticShaderUniform for FlatMaterialUniform {
  const TYPE: SemanticBinding = SemanticBinding::Material;
}

impl ShaderGraphProvider for FlatMaterialGPU {
  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let uniform = builder.register_uniform::<FlatMaterialUniform>().expand();

    builder.set_fragment_out(0, uniform.color);
    Ok(())
  }
}

impl ShaderUniformBlock for FlatMaterialUniform {
  fn shader_struct() -> &'static str {
    "
      struct FlatMaterial {
        color: vec4<f32>;
      };"
  }
}

impl BindGroupLayoutProvider for FlatMaterial {
  fn bind_preference() -> usize {
    1
  }
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: UniformBuffer::<Vec4<f32>>::bind_layout(),
        count: None,
      }],
    })
  }

  fn gen_shader_header(group: usize) -> String {
    format!(
      "
      [[group({group}), binding(0)]]
      var<uniform> flat_material: FlatMaterial;
    
    ",
    )
  }

  fn register_uniform_struct_declare(builder: &mut PipelineBuilder) {
    builder.declare_uniform_struct::<FlatMaterialUniform>();
  }
}

pub struct FlatMaterialGPU {
  _uniform: UniformBuffer<Vec4<f32>>,
  bindgroup: MaterialBindGroup,
}

impl MaterialGPUResource for FlatMaterialGPU {
  type Source = FlatMaterial;

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
          return flat_material.color;
      }}
      ",
      )
      .use_fragment_entry("fs_main");

    builder.with_layout::<FlatMaterial>(ctx.layouts, device);

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

impl MaterialCPUResource for FlatMaterial {
  type GPU = FlatMaterialGPU;

  fn create(
    &self,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
    bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> Self::GPU {
    let _uniform = UniformBuffer::create(&gpu.device, self.color);

    let bindgroup_layout = Self::layout(&gpu.device);

    let bindgroup = MaterialBindGroupBuilder::new(gpu, ctx.resources, bgw.clone())
      .push(_uniform.as_bindable())
      .build(&bindgroup_layout);

    FlatMaterialGPU {
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
