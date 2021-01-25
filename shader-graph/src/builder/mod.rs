use crate::*;

pub struct ShaderGraphBuilder;

#[allow(clippy::new_without_default)]
impl ShaderGraphBuilder {
  pub fn new() -> Self {
    let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
    *guard = Some(ShaderGraph::new());

    Self {}
  }

  pub fn set_vertex_root(&self, n: impl ShaderGraphNodeOrConst<Output = Vec4<f32>>) {
    modify_graph(|g| {
      let node =
        ShaderGraphNode::<Vec4<f32>>::new(ShaderGraphNodeData::Output(ShaderGraphOutput::Vert));
      let to_handle = g.nodes.create_node(node.into_any());
      let from_handle = unsafe { n.to_node(g).handle.cast_type() };
      g.nodes.connect_node(from_handle, to_handle);

      g.vertex_position = Some(unsafe { to_handle.cast_type().into() })
    });
  }

  pub fn set_frag_output(&self, n: impl ShaderGraphNodeOrConst<Output = Vec4<f32>>) {
    modify_graph(|g| {
      let index = g.frag_outputs.len();

      let node = ShaderGraphNode::<Vec4<f32>>::new(ShaderGraphNodeData::Output(
        ShaderGraphOutput::Frag(index),
      ));
      let to_handle = g.nodes.create_node(node.into_any());
      let from_handle = unsafe { n.to_node(g).handle.cast_type() };
      g.nodes.connect_node(from_handle, to_handle);

      g.frag_outputs
        .push((unsafe { to_handle.cast_type().into() }, index));
    });
  }

  pub fn vary<T: ShaderGraphNodeType>(&self, h: Node<T>) -> Node<T> {
    modify_graph(|graph| {
      let index = graph.varyings.len();
      let node =
        ShaderGraphNode::<T>::new(ShaderGraphNodeData::Output(ShaderGraphOutput::Vary(index)));

      let handle = graph.insert_node(node);
      graph
        .nodes
        .connect_node(unsafe { h.handle.cast_type() }, handle.handle);

      graph.varyings.push((handle, index)); // this for output, so with output type

      // this for input in fragment shader , so with input type
      let return_node =
        ShaderGraphNode::<T>::new(ShaderGraphNodeData::Input(ShaderGraphInputNode {
          node_type: ShaderGraphInputNodeType::Vary,
          name: format!("vary{}", index),
        }));
      let handle = graph.nodes.create_node(return_node.into_any());

      unsafe { handle.cast_type().into() }
    })
  }

  pub fn create(self) -> ShaderGraph {
    IN_BUILDING_SHADER_GRAPH.lock().unwrap().take().unwrap()
  }

  pub fn bindgroup<T>(&self, b: impl FnOnce(&mut ShaderGraphBindGroupBuilder) -> T) -> T {
    modify_graph(|g| {
      let mut builder = ShaderGraphBindGroupBuilder::new(g);
      let instance = b(&mut builder);
      builder.resolve();
      instance
    })
  }

  pub fn bindgroup_by<
    T: ShaderGraphBindGroupProvider + rendiation_ral::BindGroupLayoutDescriptorProvider,
  >(
    &mut self,
  ) -> T::ShaderGraphBindGroupInstance {
    modify_graph(|graph| {
      graph.shader_interface.binding_group::<T>();
    });

    self.bindgroup(|b| T::create_instance(b))
  }

  pub fn attribute<T: ShaderGraphNodeType>(&self, name: &str) -> Node<T> {
    modify_graph(|graph| {
      let data = ShaderGraphNodeData::Input(ShaderGraphInputNode {
        node_type: ShaderGraphInputNodeType::Uniform,
        name: name.to_owned(),
      });
      let node = ShaderGraphNode::<T>::new(data);
      let handle = graph.insert_node(node);
      graph.attributes.push((handle, graph.attributes.len()));
      unsafe { handle.handle.cast_type().into() }
    })
  }

  pub fn vertex_by<T: ShaderGraphGeometryProvider>(&mut self) -> T::ShaderGraphGeometryInstance {
    T::create_instance(self)
  }

  pub fn geometry_by<T: rendiation_ral::GeometryDescriptorProvider>(&mut self) {
    modify_graph(|graph| {
      graph.shader_interface.geometry::<T>();
    });
  }
}

pub struct ShaderGraphBindGroupBuilder<'a> {
  graph: &'a mut ShaderGraph,
  bindgroup: ShaderGraphBindGroup,
}

impl<'a> ShaderGraphBindGroupBuilder<'a> {
  pub fn new(graph: &'a mut ShaderGraph) -> Self {
    Self {
      graph,
      bindgroup: ShaderGraphBindGroup { inputs: Vec::new() },
    }
  }

  pub fn create_uniform_node<T: ShaderGraphNodeType>(&mut self, name: &str) -> Node<T> {
    let data = ShaderGraphNodeData::Input(ShaderGraphInputNode {
      node_type: ShaderGraphInputNodeType::Uniform,
      name: name.to_owned(),
    });
    let node = ShaderGraphNode::<T>::new(data);
    unsafe { self.graph.insert_node(node).handle.cast_type().into() }
  }

  pub fn add_none_ubo(&mut self, h: NodeUntyped, stage: ShaderStage) {
    self
      .bindgroup
      .inputs
      .push((ShaderGraphUniformInputType::NoneUBO(h), stage));
  }

  pub fn add_ubo(&mut self, info: (&'static UBOMetaInfo, Vec<NodeUntyped>), stage: ShaderStage) {
    self
      .bindgroup
      .inputs
      .push((ShaderGraphUniformInputType::UBO(info), stage));
  }

  pub fn resolve(self) {
    self.graph.bindgroups.push(self.bindgroup)
  }
}

pub struct UBOBuilder<'a, 'b> {
  bindgroup_builder: &'b mut ShaderGraphBindGroupBuilder<'a>,
  meta_info: &'static UBOMetaInfo,
  nodes: Vec<NodeUntyped>,
}

impl<'a, 'b> UBOBuilder<'a, 'b> {
  pub fn new(
    meta_info: &'static UBOMetaInfo,
    bindgroup_builder: &'b mut ShaderGraphBindGroupBuilder<'a>,
  ) -> Self {
    Self {
      bindgroup_builder,
      meta_info,
      nodes: Vec::new(),
    }
  }

  pub fn uniform<T: ShaderGraphNodeType>(&mut self, name: &str) -> Node<T> {
    let handle = self.bindgroup_builder.create_uniform_node::<T>(name);
    self.nodes.push(unsafe { handle.handle.cast_type().into() });
    handle
  }

  pub fn ok(self, stage: ShaderStage) {
    self
      .bindgroup_builder
      .add_ubo((self.meta_info, self.nodes), stage);
  }
}
