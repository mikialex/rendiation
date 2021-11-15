use std::{borrow::Cow, rc::Rc};

use crate::{BindGroupLayoutManager, BindGroupLayoutProvider, VertexBufferLayoutOwned};

pub struct PipelineBuilder {
  pub name: String,

  pub struct_declares: Vec<String>,
  pub includes: Vec<String>,

  pub vertex_entries: Vec<String>,
  pub active_vertex_entry: String,
  pub fragment_entries: Vec<String>,
  pub active_fragment_entry: String,

  pub bindgroup_declarations: Vec<String>,
  pub layouts: Vec<Rc<wgpu::BindGroupLayout>>,

  pub targets: Vec<wgpu::ColorTargetState>,
  pub depth_stencil: Option<wgpu::DepthStencilState>,
  pub vertex_input: String,
  pub vertex_buffers: Vec<VertexBufferLayoutOwned>,
  pub primitive_state: wgpu::PrimitiveState,
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
      bindgroup_declarations: Default::default(),
      struct_declares: Default::default(),
      includes: Default::default(),
      vertex_entries: Default::default(),
      fragment_entries: Default::default(),
      active_vertex_entry: Default::default(),
      active_fragment_entry: Default::default(),
      vertex_input: Default::default(),
    }
  }
}

impl PipelineBuilder {
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

  pub fn with_layout<T: BindGroupLayoutProvider>(
    &mut self,
    cache: &BindGroupLayoutManager,
    device: &wgpu::Device,
  ) -> &mut Self {
    let layout = cache.retrieve::<T>(device);
    self.layouts.push(layout.clone());
    self
      .bindgroup_declarations
      .push(T::gen_shader_header(self.bindgroup_declarations.len()));
    self
  }

  pub fn with_topology(&mut self, topology: wgpu::PrimitiveTopology) -> &mut Self {
    self.primitive_state.topology = topology;
    self
  }

  pub fn build(&self, device: &wgpu::Device) -> wgpu::RenderPipeline {
    let shader_source = format!(
      "
    {bindgroups}

    {vertex_output_struct}

    {vertex_entries}
    
    {fragment_entries}
    
    ",
      bindgroups = "",
      vertex_output_struct = "",
      vertex_entries = "",
      fragment_entries = "",
    );

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
        entry_point: "vs_main",
        buffers: vertex_buffers.as_slice(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: "fs_main",
        targets: self.targets.as_slice(),
      }),
      primitive: wgpu::PrimitiveState {
        cull_mode: None,
        topology: wgpu::PrimitiveTopology::TriangleList,
        ..Default::default()
      },
      depth_stencil: self.depth_stencil.clone(),
      multisample: wgpu::MultisampleState::default(),
    })
  }
}
