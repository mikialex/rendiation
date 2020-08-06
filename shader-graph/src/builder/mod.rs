use crate::*;

pub struct ShaderGraphBindGroup {
  pub inputs: Vec<ShaderGraphNodeHandleUntyped>,
}

/// The builder will hold the mutex guard to make sure the in building shadergraph is singleton
pub struct ShaderGraphBuilder<'a> {
  guard: MutexGuard<'a, Option<ShaderGraph>>,
}

impl<'a> ShaderGraphBuilder<'a> {
  pub fn new() -> Self {
    let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
    *guard = Some(ShaderGraph::new());

    Self { guard }
  }

  pub fn create(mut self) -> ShaderGraph {
    self.guard.take().unwrap()
  }

  pub fn bindgroup(&mut self, b: impl FnOnce(&mut ShaderGraphBindGroupBuilder)) {
    self.guard.as_mut().map(|g| {
      let mut builder = ShaderGraphBindGroupBuilder::new(g);
      b(&mut builder);
      builder.resolve();
    });
  }

  pub fn attribute<T: ShaderGraphNodeType>(&mut self, name: &str) -> ShaderGraphNodeHandle<T> {
    let data = ShaderGraphNodeData::Input(ShaderGraphInputNode {
      node_type: ShaderGraphInputNodeType::Uniform,
      name: name.to_owned(),
    });
    let graph = self.guard.as_mut().unwrap();
    let node = ShaderGraphNode::<T>::new(data);
    graph.register_type::<T>();
    let handle = graph.nodes.create_node(node.to_any());
    graph.attributes.insert((handle, graph.attributes.len()));
    unsafe { handle.cast_type() }
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
