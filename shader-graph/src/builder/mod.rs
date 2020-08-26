use crate::*;

pub struct ShaderGraphBuilder;

impl ShaderGraphBuilder {
  pub fn new() -> Self {
    let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
    *guard = Some(ShaderGraph::new());

    Self {}
  }

  pub fn set_vertex_root(&self, n: ShaderGraphNodeHandle<Vec4<f32>>) {
    modify_graph(|g| {
      let node =
        ShaderGraphNode::<Vec4<f32>>::new(ShaderGraphNodeData::Output(ShaderGraphOutput::Vert));
      let handle = g.nodes.create_node(node.to_any());
      g.nodes
        .connect_node(unsafe { n.handle.cast_type() }, handle);

      g.vertex_position = Some(unsafe { handle.cast_type().into() })
    });
  }

  pub fn set_frag_output(&self, n: ShaderGraphNodeHandle<Vec4<f32>>) {
    modify_graph(|g| {
      let index = g.frag_outputs.len();

      let node = ShaderGraphNode::<Vec4<f32>>::new(ShaderGraphNodeData::Output(
        ShaderGraphOutput::Frag(index),
      ));
      let handle = g.nodes.create_node(node.to_any());
      g.nodes
        .connect_node(unsafe { n.handle.cast_type() }, handle);
      g.frag_outputs
        .push((unsafe { handle.cast_type().into() }, index));
    });
  }

  pub fn set_vary<T: ShaderGraphNodeType>(
    &self,
    h: ShaderGraphNodeHandle<T>,
  ) -> ShaderGraphNodeHandle<T> {
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
      let handle = graph.nodes.create_node(return_node.to_any());

      unsafe { handle.cast_type().into() }
    })
  }

  pub fn create(self) -> ShaderGraph {
    IN_BUILDING_SHADER_GRAPH.lock().unwrap().take().unwrap()
  }

  pub fn bindgroup(&self, b: impl FnOnce(&mut ShaderGraphBindGroupBuilder)) {
    modify_graph(|g| {
      let mut builder = ShaderGraphBindGroupBuilder::new(g);
      b(&mut builder);
      builder.resolve();
    });
  }

  pub fn bindgroup_by<T: ShaderGraphBindGroupProvider + rendiation_webgpu::BindGroupProvider>(
    &mut self,
    layout: Arc<rendiation_webgpu::BindGroupLayout>,
  ) -> T::ShaderGraphBindGroupInstance {
    modify_graph(|graph| {
      graph.wgpu_shader_interface.binding_group::<T>(layout);
    });

    // can we do better??
    let mut re: Option<T::ShaderGraphBindGroupInstance> = None;
    self.bindgroup(|b| {
      re = Some(T::create_instance(b));
    });
    re.unwrap()
  }

  pub fn attribute<T: ShaderGraphNodeType>(&self, name: &str) -> ShaderGraphNodeHandle<T> {
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

  // create const node
  pub fn c<T: ShaderGraphNodeType + ShaderGraphConstableNodeType>(
    &self,
    value: T,
  ) -> ShaderGraphNodeHandle<T> {
    modify_graph(|graph| {
      let data = ShaderGraphNodeData::Const(Box::new(value));
      let node = ShaderGraphNode::<T>::new(data);
      unsafe { graph.insert_node(node).handle.cast_type().into() }
    })
  }

  pub fn vertex_by<T: ShaderGraphGeometryProvider>(&mut self) -> T::ShaderGraphGeometryInstance {
    T::create_instance(self)
  }

  pub fn geometry_by<T: rendiation_webgpu::GeometryProvider>(&mut self) {
    modify_graph(|graph| {
      graph.wgpu_shader_interface.geometry::<T>();
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

  pub fn create_uniform_node<T: ShaderGraphNodeType>(
    &mut self,
    name: &str,
  ) -> ShaderGraphNodeHandle<T> {
    let data = ShaderGraphNodeData::Input(ShaderGraphInputNode {
      node_type: ShaderGraphInputNodeType::Uniform,
      name: name.to_owned(),
    });
    let node = ShaderGraphNode::<T>::new(data);
    unsafe { self.graph.insert_node(node).handle.cast_type().into() }
  }

  pub fn add_none_ubo(&mut self, h: ShaderGraphNodeHandleUntyped, stage: ShaderStage) {
    self
      .bindgroup
      .inputs
      .push((ShaderGraphUniformInputType::NoneUBO(h), stage));
  }

  pub fn add_ubo(
    &mut self,
    info: (Arc<UBOInfo>, Vec<ShaderGraphNodeHandleUntyped>),
    stage: ShaderStage,
  ) {
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
  meta_info: Arc<UBOInfo>,
  nodes: Vec<ShaderGraphNodeHandleUntyped>,
}

impl<'a, 'b> UBOBuilder<'a, 'b> {
  pub fn new(
    meta_info: Arc<UBOInfo>,
    bindgroup_builder: &'b mut ShaderGraphBindGroupBuilder<'a>,
  ) -> Self {
    Self {
      bindgroup_builder,
      meta_info,
      nodes: Vec::new(),
    }
  }

  pub fn uniform<T: ShaderGraphNodeType>(&mut self, name: &str) -> ShaderGraphNodeHandle<T> {
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
