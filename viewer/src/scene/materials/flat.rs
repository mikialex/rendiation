use std::{cell::Cell, rc::Rc};

use rendiation_algebra::Vec4;
use rendiation_renderable_mesh::vertex::Vertex;
use rendiation_webgpu::*;

use crate::*;

#[derive(Clone)]
pub struct FlatMaterial {
  pub color: Vec4<f32>,
  pub states: MaterialStates,
}

impl MaterialMeshLayoutRequire for FlatMaterial {
  type VertexInput = Vec<Vertex>;
}

impl FlatMaterial {
  pub fn get_shader_header() -> &'static str {
    "
    [[block]]
    struct FlatMaterial {
      color: vec4<f32>;
    };

    [[group(1), binding(0)]]
    var<uniform> flat_material: FlatMaterial;
    
    "
  }
}

impl BindGroupLayoutProvider for FlatMaterial {
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
}

pub struct FlatMaterialGPU {
  state_id: Cell<ValueID<MaterialStates>>,
  _uniform: UniformBuffer<Vec4<f32>>,
  bindgroup: MaterialBindGroup,
}

impl PipelineRequester for FlatMaterialGPU {
  type Container = CommonPipelineCache;
}

impl MaterialGPUResource for FlatMaterialGPU {
  type Source = FlatMaterial;

  fn pipeline_key(
    &self,
    source: &Self::Source,
    ctx: &PipelineCreateCtx,
  ) -> <Self::Container as PipelineVariantContainer>::Key {
    self
      .state_id
      .set(STATE_ID.lock().unwrap().get_uuid(&source.states));
    ().key_with(self.state_id.get())
      .key_with(ctx.active_mesh.unwrap().topology())
  }
  fn create_pipeline(
    &self,
    source: &Self::Source,
    builder: &mut PipelineBuilder,
    device: &wgpu::Device,
    ctx: &PipelineCreateCtx,
  ) -> wgpu::RenderPipeline {
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
          return flat_material.color;
      }}
      
      ",
      vertex_header = Vertex::get_shader_header(),
      material_header = FlatMaterial::get_shader_header(),
      camera_header = CameraBindgroup::get_shader_header(),
      object_header = TransformGPU::get_shader_header(),
    );

    builder
      .with_layout(ctx.layouts.retrieve::<TransformGPU>(device))
      .with_layout(ctx.layouts.retrieve::<FlatMaterial>(device))
      .with_layout(ctx.layouts.retrieve::<CameraBindgroup>(device));

    builder.vertex_buffers = ctx.active_mesh.unwrap().vertex_layout();

    builder.targets = ctx
      .pass
      .color_formats
      .iter()
      .map(|&f| source.states.map_color_states(f))
      .collect();

    builder.build(device)
  }

  fn setup_pass_bindgroup<'a>(
    &self,
    pass: &mut GPURenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx,
  ) {
    pass.set_bind_group_owned(0, &ctx.model_gpu.unwrap().bindgroup, &[]);
    pass.set_bind_group_owned(1, &self.bindgroup.gpu, &[]);
    pass.set_bind_group_owned(2, &ctx.camera_gpu.bindgroup, &[]);
  }
}

impl MaterialCPUResource for FlatMaterial {
  type GPU = FlatMaterialGPU;

  fn create(
    &mut self,
    gpu: &GPU,
    _ctx: &mut SceneMaterialRenderPrepareCtx,
    bgw: &Rc<BindGroupDirtyWatcher>,
  ) -> Self::GPU {
    let _uniform = UniformBuffer::create(&gpu.device, self.color);

    let bindgroup_layout = Self::layout(&gpu.device);

    let bindgroup = MaterialBindGroupBuilder::new(gpu, bgw.clone())
      .push(_uniform.gpu().as_entire_binding())
      .build(&bindgroup_layout);

    let state_id = STATE_ID.lock().unwrap().get_uuid(&self.states);

    FlatMaterialGPU {
      state_id: Cell::new(state_id),
      _uniform,
      bindgroup,
    }
  }
}
