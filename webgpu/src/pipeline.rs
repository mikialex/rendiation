use std::{any::TypeId, borrow::Cow, collections::HashMap, rc::Rc};

use crate::{
  BindGroupLayoutCache, BindGroupLayoutProvider, ShaderUniformBlock, VertexBufferLayoutOwned,
};

pub trait ShaderBuilder {
  fn build_shader(&self) -> String;
  fn register_uniform_type<U: ShaderUniformBlock>(&mut self);
}

#[derive(Default)]
pub struct SimpleShaderBuilder {
  pub uniform_structs_decl: HashMap<TypeId, &'static str>,
  pub struct_declares: Vec<String>,
  pub includes: Vec<String>,
  pub vertex_entries: Vec<String>,
  pub active_vertex_entry: String,
  pub fragment_entries: Vec<String>,
  pub active_fragment_entry: String,
  pub bindgroup_declarations: Vec<String>,
}

impl SimpleShaderBuilder {
  pub fn include(&mut self, fun: impl Into<String>) -> &mut Self {
    self.includes.push(fun.into());
    self
  }

  pub fn declare_io_struct(&mut self, fun: impl Into<String>) -> &mut Self {
    self.struct_declares.push(fun.into());
    self
  }

  pub fn declare_uniform_struct<U: ShaderUniformBlock>(&mut self) -> &mut Self {
    self
      .uniform_structs_decl
      .insert(TypeId::of::<U>(), U::shader_struct());
    self
  }

  pub fn include_vertex_entry(&mut self, fun: impl Into<String>) -> &mut Self {
    self.vertex_entries.push(fun.into());
    self
  }

  pub fn include_fragment_entry(&mut self, fun: impl Into<String>) -> &mut Self {
    self.fragment_entries.push(fun.into());
    self
  }

  pub fn use_vertex_entry(&mut self, fun: impl Into<String>) -> &mut Self {
    self.active_vertex_entry = fun.into();
    self
  }

  pub fn use_fragment_entry(&mut self, fun: impl Into<String>) -> &mut Self {
    self.active_fragment_entry = fun.into();
    self
  }

  fn build_shader(&self) -> String {
    format!(
      "
    {uniform_struct_declares}

    {bindgroups}

    {struct_declares}

    {includes}

    {vertex_entries}
    
    {fragment_entries}
    
    ",
      bindgroups = self
        .bindgroup_declarations
        .iter()
        .map(|s| s.as_ref())
        .collect::<Vec<&str>>()
        .join("\n"),
      uniform_struct_declares = self
        .uniform_structs_decl
        .iter()
        .map(|(_, s)| *s)
        .collect::<Vec<&str>>()
        .join("\n"),
      struct_declares = self
        .struct_declares
        .iter()
        .map(|s| s.as_ref())
        .collect::<Vec<&str>>()
        .join("\n"),
      includes = self
        .includes
        .iter()
        .map(|s| s.as_ref())
        .collect::<Vec<&str>>()
        .join("\n"),
      vertex_entries = self
        .vertex_entries
        .iter()
        .map(|s| s.as_ref())
        .collect::<Vec<&str>>()
        .join("\n"),
      fragment_entries = self
        .fragment_entries
        .iter()
        .map(|s| s.as_ref())
        .collect::<Vec<&str>>()
        .join("\n"),
    )
  }
}

pub struct PipelineBuilder {
  pub name: String,
  pub shader_builder: SimpleShaderBuilder,

  pub layouts: Vec<Rc<wgpu::BindGroupLayout>>,

  pub targets: Vec<wgpu::ColorTargetState>,
  pub depth_stencil: Option<wgpu::DepthStencilState>,
  pub vertex_input: String,
  pub vertex_buffers: Vec<VertexBufferLayoutOwned>,
  pub primitive_state: wgpu::PrimitiveState,
  pub multisample: wgpu::MultisampleState,
  pub log_shader_when_finish: bool,
}

impl std::ops::Deref for PipelineBuilder {
  type Target = SimpleShaderBuilder;

  fn deref(&self) -> &Self::Target {
    &self.shader_builder
  }
}

impl std::ops::DerefMut for PipelineBuilder {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.shader_builder
  }
}

impl Default for PipelineBuilder {
  fn default() -> Self {
    Self {
      name: Default::default(),
      layouts: Default::default(),
      targets: Default::default(),
      depth_stencil: Default::default(),
      vertex_buffers: Default::default(),
      primitive_state: wgpu::PrimitiveState {
        cull_mode: None,
        topology: wgpu::PrimitiveTopology::TriangleList,
        ..Default::default()
      },
      shader_builder: Default::default(),
      vertex_input: Default::default(),
      multisample: Default::default(),
      log_shader_when_finish: false,
    }
  }
}

pub struct PlaceholderBindgroup;
impl BindGroupLayoutProvider for PlaceholderBindgroup {
  fn bind_preference() -> usize {
    0
  }

  fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: "PlaceholderBindgroup".into(),
      entries: &[],
    })
  }

  fn gen_shader_header(group: usize) -> String {
    "".to_owned()
  }

  fn register_uniform_struct_declare(builder: &mut PipelineBuilder) {}
}

impl PipelineBuilder {
  pub fn with_layout<B: BindGroupLayoutProvider>(
    &mut self,
    cache: &BindGroupLayoutCache,
    device: &wgpu::Device,
  ) -> &mut Self {
    let group_index = B::bind_preference();

    while self.layouts.len() <= group_index {
      self
        .layouts
        .push(cache.retrieve::<PlaceholderBindgroup>(device))
    }

    self.layouts[group_index] = cache.retrieve::<B>(device);

    self
      .shader_builder
      .bindgroup_declarations
      .push(B::gen_shader_header(group_index));

    B::register_uniform_struct_declare(self);

    self
  }

  pub fn with_topology(&mut self, topology: wgpu::PrimitiveTopology) -> &mut Self {
    self.primitive_state.topology = topology;
    self
  }

  pub fn build(&self, device: &wgpu::Device) -> wgpu::RenderPipeline {
    let shader_source = self.shader_builder.build_shader();

    if self.log_shader_when_finish {
      println!("{}", shader_source);
    }

    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
      label: self.name.as_str().into(),
      source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_source.as_str())),
    });

    let layouts: Vec<_> = self.layouts.iter().map(|l| l.as_ref()).collect();

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: layouts.as_slice(),
      push_constant_ranges: &[],
    });

    let vertex_buffers: Vec<_> = self.vertex_buffers.iter().map(|v| v.as_raw()).collect();

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: None,
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: self.active_vertex_entry.as_str(),
        buffers: vertex_buffers.as_slice(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: self.active_fragment_entry.as_str(),
        targets: self.targets.as_slice(),
      }),
      primitive: self.primitive_state,
      depth_stencil: self.depth_stencil.clone(),
      multisample: self.multisample,
      multiview: None,
    })
  }
}
