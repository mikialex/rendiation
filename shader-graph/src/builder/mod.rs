use crate::*;

pub struct ShaderGraphBindGroup {
  pub inputs: Vec<ShaderGraphNodeHandleUntyped>,
}

pub struct ShaderGraphBuilder;

impl ShaderGraphBuilder {
  pub fn new() -> Self {
    let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
    *guard = Some(ShaderGraph::new());

    Self {}
  }

  fn guard(&self) -> MutexGuard<'static, Option<ShaderGraph>> {
    IN_BUILDING_SHADER_GRAPH.lock().unwrap()
  }

  pub fn set_vertex_root(&self, n: ShaderGraphNodeHandle<Vec4<f32>>) {
    self.guard().as_mut().unwrap().vertex_position = Some(n)
  }

  pub fn set_frag_output(&self, n: ShaderGraphNodeHandle<Vec4<f32>>) {
    todo!()
  }

  pub fn set_vary<T: ShaderGraphNodeType>(
    &self,
    h: ShaderGraphNodeHandle<T>,
  ) -> ShaderGraphNodeHandle<T> {
    let mut g = self.guard();
    let graph = g.as_mut().unwrap();

    let index = graph.varyings.len();
    let node = ShaderGraphNode::<T>::new(ShaderGraphNodeData::Vary(index));
    graph.register_type::<T>();

    let handle = graph.nodes.create_node(node.to_any());
    graph
      .nodes
      .connect_node(unsafe { h.cast_type() }, handle);

    graph.varyings.insert((handle, index));
    unsafe { handle.cast_type() }
  }

  pub fn create(self) -> ShaderGraph {
    self.guard().take().unwrap()
  }

  pub fn bindgroup(&mut self, b: impl FnOnce(&mut ShaderGraphBindGroupBuilder)) {
    self.guard().as_mut().map(|g| {
      let mut builder = ShaderGraphBindGroupBuilder::new(g);
      b(&mut builder);
      builder.resolve();
    });
  }

  pub fn bindgroup_by<T: ShaderGraphBindGroupProvider>(
    &mut self,
  ) -> T::ShaderGraphBindGroupInstance {
    let mut re: Option<T::ShaderGraphBindGroupInstance> = None;
    self.bindgroup(|b| {
      re = Some(T::create_instance(b));
    });
    re.unwrap()
  }

  pub fn attribute<T: ShaderGraphNodeType>(&mut self, name: &str) -> ShaderGraphNodeHandle<T> {
    let data = ShaderGraphNodeData::Input(ShaderGraphInputNode {
      node_type: ShaderGraphInputNodeType::Uniform,
      name: name.to_owned(),
    });
    let mut g = self.guard();
    let graph = g.as_mut().unwrap();
    let node = ShaderGraphNode::<T>::new(data);
    graph.register_type::<T>();
    let handle = graph.nodes.create_node(node.to_any());
    graph.attributes.insert((handle, graph.attributes.len()));
    unsafe { handle.cast_type() }
  }

  pub fn geometry_by<T: ShaderGraphGeometryProvider>(&mut self) -> T::ShaderGraphGeometryInstance {
    T::create_instance(self)
  }
}

pub struct ShaderGraphBindGroupBuilder<'a> {
  index: usize,
  graph: &'a mut ShaderGraph,
  bindgroup: ShaderGraphBindGroup,
}

impl<'a> ShaderGraphBindGroupBuilder<'a> {
  pub fn new(graph: &'a mut ShaderGraph) -> Self {
    let index = graph.bindgroups.len();
    Self {
      index,
      graph,
      bindgroup: ShaderGraphBindGroup { inputs: Vec::new() },
    }
  }

  pub fn uniform<T: ShaderGraphNodeType>(&mut self, name: &str) -> ShaderGraphNodeHandle<T> {
    let data = ShaderGraphNodeData::Input(ShaderGraphInputNode {
      node_type: ShaderGraphInputNodeType::Uniform,
      name: name.to_owned(),
    });
    let node = ShaderGraphNode::<T>::new(data);
    let handle = self.graph.nodes.create_node(node.to_any());
    self.graph.register_type::<T>();
    self.graph.uniforms.insert((handle, self.index));
    self.bindgroup.inputs.push(handle);
    unsafe { handle.cast_type() }
  }

  pub fn resolve(self) {
    self.graph.bindgroups.push(self.bindgroup)
  }
}
