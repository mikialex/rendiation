use crate::*;

/// Descriptor of the shader input
#[derive(Clone)]
pub struct PipelineShaderInterfaceInfo {
  pub bindgroup_layouts: Vec<Vec<BindGroupLayoutEntry>>,
  pub vertex_state: Option<Vec<VertexBufferLayout<'static>>>,
  pub primitive_topology: PrimitiveTopology,
  pub preferred_target_states: TargetStates,
}

impl Default for PipelineShaderInterfaceInfo {
  fn default() -> Self {
    Self::new()
  }
}

impl PipelineShaderInterfaceInfo {
  pub fn new() -> Self {
    Self {
      bindgroup_layouts: Vec::new(),
      vertex_state: None,
      primitive_topology: PrimitiveTopology::TriangleList,
      preferred_target_states: TargetStates::default(),
    }
  }

  pub fn binding_group<T: BindGroupLayoutDescriptorProvider>(&mut self) -> &mut Self {
    self.bindgroup_layouts.push(T::create_descriptor());
    self
  }

  pub fn geometry<T: GeometryDescriptorProvider>(&mut self) -> &mut Self {
    self.vertex_state = Some(T::create_descriptor());
    self.primitive_topology = T::get_primitive_topology();
    self
  }
}

pub struct BindGroupLayoutBuilder {
  pub bindings: Vec<BindGroupLayoutEntry>,
}

impl BindGroupLayoutBuilder {
  pub fn new() -> Self {
    Self {
      bindings: Vec::new(),
    }
  }

  pub fn bind<T: BindGroupLayoutEntryProvider>(mut self, visibility: ShaderStage) -> Self {
    let binding = self.bindings.len() as u32;
    self
      .bindings
      .push(T::create_layout_entry(binding, visibility));
    self
  }

  pub fn build(self) -> Vec<BindGroupLayoutEntry> {
    self.bindings
  }
}
