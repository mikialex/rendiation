use std::borrow::Cow;

use rendiation_algebra::*;
use rendiation_webgpu::*;

use crate::*;

#[derive(Clone)]
pub struct FatLineMaterial {
  pub width: f32,
  pub states: MaterialStates,
}

pub struct FatLineMaterialGPU {
  uniform: UniformBuffer<f32>,
  bindgroup: MaterialBindGroup,
}

impl MaterialMeshLayoutRequire for FatLineMaterial {
  type VertexInput = Vec<FatLineVertex>;
}

pub struct FatLineVertex {
  start: Vec3<f32>,
  end: Vec3<f32>,
  color: Vec3<f32>,
}

impl VertexBufferSourceType for FatLineVertex {
  fn vertex_layout() -> VertexBufferLayout<'static> {
    VertexBufferLayout {
      array_stride: std::mem::size_of::<Self>() as u64,
      step_mode: VertexStepMode::Instance,
      attributes: &[
        VertexAttribute {
          format: VertexFormat::Float32x3,
          offset: 0,
          shader_location: 0,
        },
        VertexAttribute {
          format: VertexFormat::Float32x3,
          offset: 4 * 3,
          shader_location: 1,
        },
        VertexAttribute {
          format: VertexFormat::Float32x3,
          offset: 4 * 3 + 4 * 3,
          shader_location: 2,
        },
      ],
    }
  }

  fn get_shader_header() -> &'static str {
    r#"
      [[location(1)]] fatline_start: vec3<f32>,
      [[location(2)]] fatline_end: vec3<f32>,
      [[location(3)]] fatline_color: vec3<f32>,
    "#
  }
}

pub struct FatlineMaterialGPU {
  state_id: ValueID<MaterialStates>,
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

  pub fn create_pipeline(&self, gpu: &GPU, ctx: &PipelineCreateCtx) -> wgpu::RenderPipeline {
    let bindgroup_layout = Self::create_bindgroup_layout(&gpu.device);

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

    let shader = gpu
      .device
      .create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source.as_str())),
      });

    let pipeline_layout = gpu
      .device
      .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[
          ctx.layouts.retrieve::<TransformGPU>(),
          &bindgroup_layout,
          ctx.layouts.retrieve::<CameraBindgroup>(),
        ],
        push_constant_ranges: &[],
      });

    let vertex_buffers = ctx.active_mesh.unwrap().vertex_layout();

    let targets: Vec<_> = ctx
      .pass
      .color_format()
      .iter()
      .map(|&f| self.states.map_color_states(f))
      .collect();

    gpu
      .device
      .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
          .map_depth_stencil_state(ctx.pass.depth_stencil_format()),
        multisample: wgpu::MultisampleState::default(),
      })
  }
}

impl MaterialGPUResource for FatlineMaterialGPU {
  type Source = FatLineMaterial;

  fn request_pipeline(
    &mut self,
    source: &Self::Source,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) {
    self.state_id = STATE_ID.lock().unwrap().get_uuid(source.states);

    let key = CommonPipelineVariantKey(self.state_id, ctx.active_mesh.unwrap().topology());

    let (pipelines, pipeline_ctx) = ctx.pipeline_ctx();

    pipelines
      .get_cache_mut::<Self, CommonPipelineCache>()
      .request(&key, || source.create_pipeline(gpu, &pipeline_ctx));
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a>,
  ) {
    let key = CommonPipelineVariantKey(self.state_id, ctx.active_mesh.unwrap().topology());

    let pipeline = ctx
      .pipelines
      .get_cache::<Self, CommonPipelineCache>()
      .retrieve(&key);

    pass.set_pipeline(pipeline);
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
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) -> Self::GPU {
    let _uniform = UniformBuffer::create(&gpu.device, self.width);

    let bindgroup_layout = Self::create_bindgroup_layout(&gpu.device);
    let bindgroup = self.create_bindgroup(handle, _uniform.gpu(), &gpu.device, &bindgroup_layout);

    let state_id = STATE_ID.lock().unwrap().get_uuid(self.states);

    FatlineMaterialGPU {
      state_id,
      _uniform,
      bindgroup,
    }
  }
}
