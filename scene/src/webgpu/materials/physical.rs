use std::rc::Rc;

use rendiation_algebra::Vec3;
use rendiation_renderable_mesh::vertex::Vertex;
use rendiation_webgpu::*;

use crate::*;

impl MaterialMeshLayoutRequire for PhysicalMaterial {
  type VertexInput = Vec<Vertex>;
}

pub struct PhysicalMaterialUniform {
  pub albedo: Vec3<f32>,
}

impl ShaderUniformBlock for PhysicalMaterialUniform {
  fn shader_struct() -> &'static str {
    "
    struct PhysicalMaterial {
      albedo: vec3<f32>;
    };
    "
  }
}

impl BindGroupLayoutProvider for PhysicalMaterial {
  fn bind_preference() -> usize {
    1
  }
  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::VERTEX,
          ty: UniformBuffer::<Vec3<f32>>::bind_layout(),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: WebGPUTexture2d::bind_layout(),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 2,
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
      var<uniform> material: PhysicalMaterial;
      
      [[group({group}), binding(1)]]
      var material_albedo: texture_2d<f32>;

      [[group({group}), binding(2)]]
      var r_sampler: sampler;
    
    "
    )
  }

  fn register_uniform_struct_declare(builder: &mut PipelineBuilder) {
    builder.declare_uniform_struct::<PhysicalMaterialUniform>();
  }
}

pub struct PhysicalMaterialGPU {
  _uniform: UniformBuffer<Vec3<f32>>,
  bindgroup: MaterialBindGroup,
}

impl MaterialGPUResource for PhysicalMaterialGPU {
  type Source = PhysicalMaterial;

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
          return textureSample(material_albedo, r_sampler, in.uv);
      }}
      ",
      )
      .use_fragment_entry("fs_main");

    builder.with_layout::<PhysicalMaterial>(ctx.layouts, device);

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

impl MaterialCPUResource for PhysicalMaterial {
  type GPU = PhysicalMaterialGPU;

  fn create(
    &mut self,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
    bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> Self::GPU {
    let _uniform = UniformBuffer::create(&gpu.device, self.albedo);

    let bindgroup_layout = Self::layout(&gpu.device); // todo remove

    let sampler = ctx.map_sampler(self.sampler, &gpu.device);
    let bindgroup = MaterialBindGroupBuilder::new(gpu, bgw.clone())
      .push(_uniform.as_bindable())
      .push_texture(&self.texture)
      .push(sampler.as_bindable())
      .build(&bindgroup_layout);

    PhysicalMaterialGPU {
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
