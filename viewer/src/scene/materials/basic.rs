use std::{borrow::Cow, sync::Mutex};

use rendiation_algebra::Vec3;
use rendiation_renderable_mesh::vertex::Vertex;

use crate::{
  renderer::{BindableResource, Renderer, UniformBuffer},
  scene::{
    BindGroup, CameraBindgroup, MaterialHandle, Mesh, ModelTransformGPU, SamplerHandle,
    SceneTexture2dGpu, StandardForward, Texture2DHandle, ValueID, ValueIDGenerator,
    VertexBufferSourceType, ViewerDeviceExt,
  },
};

use super::{
  MaterialCPUResource, MaterialGPUResource, PipelineResourceManager, SceneMaterialPassSetupCtx,
  SceneMaterialRenderPrepareCtx,
};

pub struct BasicMaterial {
  pub color: Vec3<f32>,
  pub sampler: SamplerHandle,
  pub texture: Texture2DHandle,
  pub states: PreferredMaterialStates,
}

impl BasicMaterial {
  pub fn create_bindgroup<S>(
    &self,
    handle: MaterialHandle,
    ubo: &wgpu::Buffer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    ctx: &mut SceneMaterialRenderPrepareCtx<S>,
  ) -> BindGroup {
    device
      .material_bindgroup_builder(handle)
      .push(ubo.as_entire_binding())
      .push_texture2d(ctx, self.texture)
      .push_sampler(ctx, self.sampler)
      .build(layout)
  }

  pub fn get_shader_header() -> &'static str {
    "
    [[block]]
    struct BasicMaterial {
      color: vec3<f32>;
    };

    [[group(1), binding(0)]]
    var basic_material: BasicMaterial;
    
    [[group(1), binding(1)]]
    var r_color: texture_2d<f32>;

    [[group(1), binding(2)]]
    var r_sampler: sampler;
    "
  }

  pub fn create_bindgroup_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStage::VERTEX,
          ty: UniformBuffer::<Vec3<f32>>::bind_layout(),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStage::FRAGMENT,
          ty: SceneTexture2dGpu::bind_layout(),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 2,
          visibility: wgpu::ShaderStage::FRAGMENT,
          ty: wgpu::Sampler::bind_layout(),
          count: None,
        },
      ],
    })
  }
}

pub struct BasicMaterialGPU {
  state_id: ValueID<PreferredMaterialStates>,
  uniform: UniformBuffer<Vec3<f32>>,
  bindgroup_layout: wgpu::BindGroupLayout,
  bindgroup: BindGroup,
}

impl MaterialGPUResource<StandardForward> for BasicMaterialGPU {
  type Source = BasicMaterial;
  fn update(
    &mut self,
    source: &Self::Source,
    renderer: &Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<StandardForward>,
  ) {
    //
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx<'a, StandardForward>,
  ) {
    let pipeline = ctx.pipelines.basic.as_ref().unwrap();
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &ctx.model_gpu.bindgroup, &[]);
    pass.set_bind_group(1, &self.bindgroup.gpu, &[]);
    pass.set_bind_group(2, &ctx.camera_gpu.bindgroup, &[]);
  }
}

impl MaterialCPUResource for BasicMaterial {
  type GPU = BasicMaterialGPU;

  fn create<S>(
    &mut self,
    handle: MaterialHandle,
    renderer: &mut Renderer,
    ctx: &mut SceneMaterialRenderPrepareCtx<S>,
  ) -> Self::GPU {
    let uniform = UniformBuffer::create(&renderer.device, self.color);

    let bindgroup_layout = Self::create_bindgroup_layout(&renderer.device);
    let bindgroup = self.create_bindgroup(
      handle,
      uniform.gpu(),
      &renderer.device,
      &renderer.queue,
      &bindgroup_layout,
      ctx,
    );

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
        out.position = camera.projection * camera.view * model.matrix * vec4<f32>(position, 1.0);;
        return out;
      }}
      
      [[stage(fragment)]]
      fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {{
          return textureSample(r_color, r_sampler, in.uv);
      }}
      
      ",
      vertex_header = Vertex::get_shader_header(),
      material_header = Self::get_shader_header(),
      camera_header = CameraBindgroup::get_shader_header(),
      object_header = ModelTransformGPU::get_shader_header(),
    );

    let shader = renderer
      .device
      .create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source.as_str())),
        flags: renderer.create_shader_flags(),
      });

    let pipeline_layout = renderer
      .device
      .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[
          &ctx.model_gpu.layout,
          &bindgroup_layout,
          &ctx.camera_gpu.layout,
        ],
        push_constant_ranges: &[],
      });

    let vertex_buffers = ctx.active_mesh.vertex_layout();
    let pipeline = renderer
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
          targets: &[self
            .states
            .map_color_states(renderer.get_prefer_target_format())],
        }),
        primitive: wgpu::PrimitiveState {
          cull_mode: None,
          topology: wgpu::PrimitiveTopology::TriangleList,
          ..Default::default()
        },
        depth_stencil: self
          .states
          .map_depth_stencil_state(StandardForward::depth_format().into()),
        multisample: wgpu::MultisampleState::default(),
      });

    ctx.pipelines.basic = pipeline.into();

    let state_id = STATE_ID.lock().unwrap().get_uuid(self.states);

    BasicMaterialGPU {
      state_id,
      uniform,
      bindgroup_layout,
      bindgroup,
    }
  }
}

static STATE_ID: once_cell::sync::Lazy<Mutex<ValueIDGenerator<PreferredMaterialStates>>> =
  once_cell::sync::Lazy::new(|| Mutex::new(ValueIDGenerator::default()));

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PreferredMaterialStates {
  pub depth_write_enabled: bool,
  pub depth_compare: wgpu::CompareFunction,
  // pub stencil: wgpu::StencilState,
  // pub bias: Default::default(),
  pub blend: Option<wgpu::BlendState>,
  pub write_mask: wgpu::ColorWrite,
}

impl PreferredMaterialStates {
  fn map_color_states(&self, format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
      format,
      blend: self.blend,
      write_mask: self.write_mask,
    }
  }
  fn map_depth_stencil_state(
    &self,
    format: Option<wgpu::TextureFormat>,
  ) -> Option<wgpu::DepthStencilState> {
    format.map(|format| wgpu::DepthStencilState {
      format,
      depth_write_enabled: self.depth_write_enabled,
      depth_compare: self.depth_compare,
      stencil: Default::default(),
      bias: Default::default(),
    })
  }
}

impl Default for PreferredMaterialStates {
  fn default() -> Self {
    Self {
      depth_write_enabled: true,
      depth_compare: wgpu::CompareFunction::Less,
      blend: None,
      write_mask: wgpu::ColorWrite::all(),
    }
  }
}

pub struct PipelineTopologyVariant {}

pub trait MaterialPipelineStorage<M> {
  fn request(&mut self, material: &M, mesh: &dyn Mesh);
  fn retrieve(&self) -> &wgpu::RenderPipeline;
}

impl MaterialPipelineStorage<BasicMaterial> for PipelineResourceManager {
  fn request(&mut self, material: &BasicMaterial, mesh: &dyn Mesh) {
    todo!()
  }

  fn retrieve(&self) -> &wgpu::RenderPipeline {
    todo!()
  }
}
