use rendiation_webgpu::*;
use std::{borrow::Cow, cell::Cell};

use crate::*;

#[derive(Clone)]
pub struct FatLineMaterial {
  pub width: f32,
  pub states: MaterialStates,
}

pub struct FatlineMaterialGPU {
  state_id: Cell<ValueID<MaterialStates>>,
  _uniform: UniformBuffer<f32>,
  bindgroup: MaterialBindGroup,
}

impl FatLineMaterial {
  pub fn create_bindgroup(
    &self,
    handle: MaterialHandle,
    ubo: &wgpu::Buffer,
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
  ) -> MaterialBindGroup {
    device
      .material_bindgroup_builder(handle)
      .push(ubo.as_entire_binding())
      .build(layout)
  }

  pub fn create_bindgroup_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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

  pub fn get_shader_header() -> &'static str {
    "
    [[block]]
    struct FatlineMaterial {
      width: f32;
    };

    [[group(1), binding(0)]]
    var fatline_material: FatlineMaterial;
    "
  }

  pub fn create_pipeline(
    &self,
    device: &wgpu::Device,
    ctx: &PipelineCreateCtx,
  ) -> wgpu::RenderPipeline {
    let bindgroup_layout = Self::create_bindgroup_layout(device);

    let shader_source = format!(
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
      material_header = Self::get_shader_header(),
      camera_header = CameraBindgroup::get_shader_header(),
      object_header = TransformGPU::get_shader_header(),
    );

    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
      label: None,
      source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source.as_str())),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: &[
        ctx.layouts.retrieve::<TransformGPU>(device),
        &bindgroup_layout,
        ctx.layouts.retrieve::<CameraBindgroup>(device),
      ],
      push_constant_ranges: &[],
    });

    let vertex_buffers = ctx.active_mesh.unwrap().vertex_layout();

    let targets: Vec<_> = ctx
      .pass
      .color_formats
      .iter()
      .map(|&f| self.states.map_color_states(f))
      .collect();

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: None,
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: "vs_main",
        buffers: &vertex_buffers,
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: "fs_main",
        targets: targets.as_slice(),
      }),
      primitive: wgpu::PrimitiveState {
        cull_mode: None,
        topology: wgpu::PrimitiveTopology::TriangleList,
        ..Default::default()
      },
      depth_stencil: self
        .states
        .map_depth_stencil_state(ctx.pass.depth_stencil_format),
      multisample: wgpu::MultisampleState::default(),
    })
  }
}

impl PipelineRequester for FatlineMaterialGPU {
  type Container = CommonPipelineCache;
  type Key = CommonPipelineVariantKey;
}

impl MaterialGPUResource for FatlineMaterialGPU {
  type Source = FatLineMaterial;

  fn pipeline_key(&self, source: &Self::Source, ctx: &PipelineCreateCtx) -> Self::Key {
    self
      .state_id
      .set(STATE_ID.lock().unwrap().get_uuid(source.states));
    CommonPipelineVariantKey(self.state_id.get(), ctx.active_mesh.unwrap().topology())
  }
  fn create_pipeline(
    &self,
    source: &Self::Source,
    device: &wgpu::Device,
    ctx: &PipelineCreateCtx,
  ) -> wgpu::RenderPipeline {
    source.create_pipeline(device, ctx)
  }

  fn setup_pass_bindgroup<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a>,
  ) {
    pass.set_bind_group(0, &ctx.model_gpu.unwrap().bindgroup, &[]);
    pass.set_bind_group(1, &self.bindgroup.gpu, &[]);
    pass.set_bind_group(2, &ctx.camera_gpu.bindgroup, &[]);
  }

  fn update(
    &mut self,
    _source: &Self::Source,
    _gpu: &GPU,
    _ctx: &mut SceneMaterialRenderPrepareCtx,
    _bindgroup_changed: bool,
  ) -> bool {
    true
  }
}

impl MaterialCPUResource for FatLineMaterial {
  type GPU = FatlineMaterialGPU;

  fn create(
    &mut self,
    handle: MaterialHandle,
    gpu: &GPU,
    _ctx: &mut SceneMaterialRenderPrepareCtx,
  ) -> Self::GPU {
    let device = &gpu.device;
    let _uniform = UniformBuffer::create(device, self.width);

    let bindgroup_layout = Self::create_bindgroup_layout(device);
    let bindgroup = self.create_bindgroup(handle, _uniform.gpu(), device, &bindgroup_layout);

    let state_id = STATE_ID.lock().unwrap().get_uuid(self.states);

    FatlineMaterialGPU {
      state_id: Cell::new(state_id),
      _uniform,
      bindgroup,
    }
  }
}
