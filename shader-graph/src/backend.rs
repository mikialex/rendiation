use rendiation_ral::*;

pub struct ShaderInterfaceInfo<T: ShaderGraphBackend> {
  bindgroup_layouts: Vec<T::BindGroupLayoutDescriptor>,
  vertex_state: Option<T::VertexStateDescriptor>,
  primitive_topology: PrimitiveTopology,
  color_states: Vec<T::ColorStateDescriptor>,
  depth_stencil_state: Vec<T::ColorStateDescriptor>,
}

impl<T: ShaderGraphBackend> ShaderInterfaceInfo<T> {
  pub fn new() -> Self {
    Self {
      bindgroup_layouts: Vec::new(),
      vertex_state: None,
      primitive_topology: PrimitiveTopology::TriangleList,
      color_states: Vec::new(),
      depth_stencil_state: Vec::new(),
    }
  }

  pub fn binding_group<T: WGPUBindGroupLayoutProvider>(
    &mut self,
    layout: Arc<wgpu::BindGroupLayout>,
  ) -> &mut Self {
    self.bindgroup_layouts.push(layout.clone());
    self
  }

  pub fn geometry<T: WGPUGeometryProvider>(&mut self) -> &mut Self {
    self.vertex_state = Some(T::get_geometry_vertex_state_descriptor());
    self.primitive_topology = T::get_primitive_topology();
    self
  }
}

pub struct ShaderGraphOutput<T: ShaderGraphBackend> {
  vertex_shader: String, // maybe this will become naga ir someday
  frag_shader: String,
  shader_interface: ShaderInterfaceInfo<T>,
}

pub trait ShaderGraphBackend: RAL {
  type BindGroupLayoutDescriptor;
  type VertexStateDescriptor;
  type ColorStateDescriptor;
  type DepthStencilStateDescriptor;
  // type
  fn convert_to_shader_build_source(output: &ShaderGraphOutput<Self>) -> Self::ShaderBuildSource;
}

pub struct AnyBackend;

impl RAL for AnyBackend {
  type RenderTarget = ();
  type RenderPass = ();
  type Renderer = ();
  type ShaderBuildSource = ();
  type Shading = ();
  type BindGroup = ();
  type IndexBuffer = ();
  type VertexBuffer = ();
  type UniformBuffer = ();
  type Texture = ();
  type Sampler = ();

  fn create_shading(renderer: &mut Self::Renderer, des: &Self::ShaderBuildSource) -> Self::Shading {
    todo!()
  }

  fn dispose_shading(renderer: &mut Self::Renderer, shading: Self::Shading) {
    todo!()
  }

  fn apply_shading(pass: &mut Self::RenderPass, shading: &Self::Shading) {
    todo!()
  }

  fn apply_bindgroup(pass: &mut Self::RenderPass, index: usize, bindgroup: &Self::BindGroup) {
    todo!()
  }

  fn apply_vertex_buffer(pass: &mut Self::RenderPass, index: i32, vertex: &Self::VertexBuffer) {
    todo!()
  }

  fn apply_index_buffer(pass: &mut Self::RenderPass, index: &Self::IndexBuffer) {
    todo!()
  }

  fn create_uniform_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::UniformBuffer {
    todo!()
  }

  fn dispose_uniform_buffer(renderer: &mut Self::Renderer, uniform: Self::UniformBuffer) {
    todo!()
  }

  fn update_uniform_buffer(
    renderer: &mut Self::Renderer,
    gpu: &mut Self::UniformBuffer,
    data: &[u8],
    range: std::ops::Range<usize>,
  ) {
    todo!()
  }

  fn create_index_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::IndexBuffer {
    todo!()
  }

  fn dispose_index_buffer(renderer: &mut Self::Renderer, buffer: Self::IndexBuffer) {
    todo!()
  }

  fn create_vertex_buffer(
    renderer: &mut Self::Renderer,
    data: &[u8],
    layout: RALVertexBufferDescriptor,
  ) -> Self::VertexBuffer {
    todo!()
  }

  fn dispose_vertex_buffer(renderer: &mut Self::Renderer, buffer: Self::VertexBuffer) {
    todo!()
  }

  fn set_viewport(pass: &mut Self::RenderPass, viewport: &Viewport) {
    todo!()
  }

  fn draw_indexed(
    pass: &mut Self::RenderPass,
    topology: PrimitiveTopology,
    range: std::ops::Range<u32>,
  ) {
    todo!()
  }

  fn draw_none_indexed(
    pass: &mut Self::RenderPass,
    topology: PrimitiveTopology,
    range: std::ops::Range<u32>,
  ) {
    todo!()
  }

  fn render_drawcall<G: GeometryProvider<Self>, SP: ShadingProvider<Self, Geometry = G>>(
    drawcall: &Drawcall<Self, G, SP>,
    pass: &mut Self::RenderPass,
    resources: &ResourceManager<Self>,
  ) {
    todo!()
  }
}

impl ShaderGraphBackend for AnyBackend {
  type BindGroupLayoutDescriptor = ();
  type VertexStateDescriptor = ();
  type ColorStateDescriptor = ();
  type DepthStencilStateDescriptor = ();
  fn convert_to_shader_build_source(output: &ShaderGraphOutput<Self>) -> Self::ShaderBuildSource {
    todo!()
  }
}
