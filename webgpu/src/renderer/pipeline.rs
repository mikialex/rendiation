use crate::{render_target::TargetStates, WGPURenderer};
use std::{rc::Rc, any::TypeId};

pub struct WGPUPipeline {
  pub pipeline: wgpu::RenderPipeline,
}

pub trait VertexProvider {
  fn get_buffer_layout_descriptor() -> wgpu::VertexBufferDescriptor<'static>;
}
pub trait GeometryProvider {
  fn get_geometry_vertex_state_descriptor() -> wgpu::VertexStateDescriptor<'static>;
  fn get_primitive_topology() -> wgpu::PrimitiveTopology;
}

pub trait BindGroupProvider: Sized + 'static{
  fn provide_layout(renderer: &WGPURenderer) -> wgpu::BindGroupLayout;
}

pub struct PipelineBuilder<'a> {
  renderer: &'a WGPURenderer,
  vertex_shader: Vec<u32>,
  frag_shader: Vec<u32>,
  bindgroup_layouts: Vec<Rc<wgpu::BindGroupLayout>>,
  vertex_state: Option<wgpu::VertexStateDescriptor<'static>>,
  target_states: TargetStates,
  rasterization: wgpu::RasterizationStateDescriptor,
  primitive_topology: wgpu::PrimitiveTopology,
}

impl<'a> AsMut<Self> for PipelineBuilder<'a> {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}

impl<'a> PipelineBuilder<'a> {
  pub fn new(renderer: &'a WGPURenderer, vertex_shader: Vec<u32>, frag_shader: Vec<u32>) -> Self {
    Self {
      renderer,
      vertex_shader,
      frag_shader,
      bindgroup_layouts: Vec::new(),
      vertex_state: None,
      rasterization: wgpu::RasterizationStateDescriptor {
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: wgpu::CullMode::None,
        depth_bias: 0,
        depth_bias_slope_scale: 0.0,
        depth_bias_clamp: 0.0,
      },
      target_states: TargetStates::default(),
      primitive_topology: wgpu::PrimitiveTopology::TriangleList,
    }
  }

  pub fn binding_group<T: BindGroupProvider>(&mut self) -> &mut Self {
    let id = TypeId::of::<T>();
    let mut cache = self.renderer.bindgroup_layout_cache.borrow_mut();
    let layout = cache.entry(id)
    .or_insert_with(||{
      Rc::new(T::provide_layout(self.renderer))
    }).clone();
    self
      .bindgroup_layouts
      .push(layout);
    self
  }

  pub fn geometry<T: GeometryProvider>(&mut self) -> &mut Self {
    self.vertex_state = Some(T::get_geometry_vertex_state_descriptor());
    self.primitive_topology = T::get_primitive_topology();
    self
  }

  pub fn target_states(&mut self, states: &TargetStates) -> &mut Self {
    self.target_states = states.clone();
    self
  }

  pub fn build(&self) -> WGPUPipeline {
    let device = &self.renderer.device;
    let bind_group_layouts: Vec<_> = self.bindgroup_layouts.iter().map(|l|l.as_ref()).collect();
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      bind_group_layouts: &bind_group_layouts,
    });

    // Create the render pipeline
    let vs_module = device.create_shader_module(&self.vertex_shader);
    let fs_module = device.create_shader_module(&self.frag_shader);

    let pipeline_des = wgpu::RenderPipelineDescriptor {
      layout: &pipeline_layout,

      vertex_stage: wgpu::ProgrammableStageDescriptor {
        module: &vs_module,
        entry_point: "main",
      },
      fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
        module: &fs_module,
        entry_point: "main",
      }),

      color_states: &self.target_states.color_states,
      depth_stencil_state: self.target_states.depth_state.to_owned(),

      primitive_topology: self.primitive_topology,
      vertex_state: self.vertex_state.to_owned().unwrap(),
      sample_count: 1,
      sample_mask: !0,
      alpha_to_coverage_enabled: false,
      rasterization_state: Some(self.rasterization.clone()),
    };

    let pipeline = device.create_render_pipeline(&pipeline_des);

    WGPUPipeline { pipeline }
  }
}
