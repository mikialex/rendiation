use crate::{render_target::TargetStates, WGPUBindGroup, WGPURenderer};

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

pub trait BindGroupProvider: Sized {
  fn provide_layout(renderer: &WGPURenderer) -> &'static wgpu::BindGroupLayout;
  fn create_bindgroup(&self, renderer: &WGPURenderer) -> WGPUBindGroup;
}

pub struct PipelineBuilder<'a> {
  renderer: &'a WGPURenderer,
  vertex_shader: &'static str,
  frag_shader: &'static str,
  bindgroup_layouts: Vec<&'static wgpu::BindGroupLayout>,
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
  pub fn new(
    renderer: &'a WGPURenderer,
    vertex_shader: &'static str,
    frag_shader: &'static str,
  ) -> Self {
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
    self
      .bindgroup_layouts
      .push(T::provide_layout(self.renderer));
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
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      bind_group_layouts: &self.bindgroup_layouts,
    });

    // Create the render pipeline
    use crate::renderer::shader_util::*;
    let vs_bytes = load_glsl(&self.vertex_shader, ShaderType::Vertex);
    let fs_bytes = load_glsl(&self.frag_shader, ShaderType::Fragment);
    let vs_module = device.create_shader_module(&vs_bytes);
    let fs_module = device.create_shader_module(&fs_bytes);

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
      // index_format: self.index_format,
      // vertex_buffers: &self.vertex_layouts,
      sample_count: 1,
      sample_mask: !0,
      alpha_to_coverage_enabled: false,
      rasterization_state: Some(self.rasterization.clone()),
    };

    let pipeline = device.create_render_pipeline(&pipeline_des);

    WGPUPipeline { pipeline }
  }
}
