use crate::*;

impl OperatorNode {
  pub fn insert_graph<T: ShaderGraphNodeType>(self) -> Node<T> {
    ShaderGraphNodeExpr::Operator(self).insert_graph()
  }
}

impl ShaderGraphInputNode {
  pub fn insert_graph<T: ShaderGraphNodeType>(self) -> Node<T> {
    ShaderGraphNode::Input(self).insert_graph()
  }
}

impl ShaderGraphNodeExpr {
  pub fn insert_graph<T: ShaderGraphNodeType>(self) -> Node<T> {
    modify_graph(|graph| self.insert_into_graph(graph))
  }

  pub fn insert_into_graph<T: ShaderGraphNodeType>(
    self,
    builder: &mut ShaderGraphBuilder,
  ) -> Node<T> {
    ShaderGraphNode::Expr(self).insert_into_graph(builder)
  }
}

fn get_current_graph_stack_bottom() -> usize {
  modify_graph(|graph| {
    graph.scopes[0].graph_guid // scope 0 should always exist
  })
}

impl ShaderSideEffectNode {
  pub fn insert_graph_bottom(self) {
    self.insert_graph(get_current_graph_stack_bottom());
  }
  pub fn insert_graph(self, target_scope_id: usize) {
    modify_graph(|graph| {
      let node = ShaderGraphNode::SideEffect(self).insert_into_graph::<AnyType>(graph);
      let mut find_target_scope = false;
      for scope in &mut graph.scopes {
        if scope.graph_guid == target_scope_id {
          find_target_scope = true;
        }
        if find_target_scope {
          scope.has_side_effect = true;
        }
      }
      assert!(find_target_scope);
      let top = graph.top_scope_mut();
      let nodes = &mut top.nodes;
      top
        .inserted
        .iter()
        .take(top.inserted.len() - 1)
        .for_each(|n| nodes.connect_node(n.handle, node.handle().handle));
      top.barriers.push(node.handle());
    })
  }
}

impl ShaderControlFlowNode {
  pub fn has_side_effect(&self) -> bool {
    match self {
      ShaderControlFlowNode::If { scope, .. } => scope.has_side_effect,
      ShaderControlFlowNode::For { scope, .. } => scope.has_side_effect,
    }
  }
  pub fn collect_captured(&self) -> Vec<ShaderGraphNodeRawHandle> {
    match self {
      ShaderControlFlowNode::If { scope, .. } => scope.captured.clone(),
      ShaderControlFlowNode::For { scope, .. } => scope.captured.clone(),
    }
  }
  pub fn collect_writes(&self) -> Vec<(Rc<PendingResolve>, ShaderGraphNodeRawHandle)> {
    match self {
      ShaderControlFlowNode::If { scope, .. } => scope.writes.clone(),
      ShaderControlFlowNode::For { scope, .. } => scope.writes.clone(),
    }
  }
  pub fn insert_into_graph(self, builder: &mut ShaderGraphBuilder) {
    let has_side_effect = self.has_side_effect();
    let captured = self.collect_captured();
    let writes = self.collect_writes();
    let node = ShaderGraphNode::ControlFlow(self).insert_into_graph::<AnyType>(builder);
    let top = builder.top_scope_mut();
    let nodes = &mut top.nodes;

    if has_side_effect {
      top
        .inserted
        .iter()
        .take(top.inserted.len() - 1)
        .for_each(|n| {
          let d = nodes.get_node(n.handle).data();
          if let ShaderGraphNode::Write { .. } = d.node {
            nodes.connect_node(n.handle, node.handle().handle)
          }
        });
      top.barriers.push(node.handle());
    }

    // visit all the captured node in this scope generate before, and check
    // if it's same and generate dep, if not, pass the captured to parent scope
    for captured in captured {
      let mut find_captured = false;
      for &n in top.inserted.iter().take(top.inserted.len() - 1) {
        if captured == n {
          nodes.connect_node(n.handle, node.handle().handle);
          find_captured = true;
          break;
        }
      }
      if !find_captured {
        top.captured.push(captured);
      }
    }

    // visit all the captured write nodes in this scope generated before, and check
    // if it's same and generate dep and a write node. if not same, pass the captured nodes
    // to the parent scope
    for write in writes {
      let im_write = ShaderGraphNode::Write {
        old: None,
        new: write.1,
      }
      .insert_into_graph_inner::<AnyType>(top, ShaderValueType::Never);
      top
        .nodes
        .connect_node(node.handle().handle, im_write.handle().handle);

      write.0.current.set(im_write.handle());

      let mut find_write = false;
      for &n in top.inserted.iter().take(top.inserted.len() - 1) {
        if write.1 == n {
          find_write = true;
          break;
        }
      }
      if !find_write {
        top.writes.push(write);
      }
    }
  }
}

impl ShaderGraphNode {
  pub fn insert_graph<T: ShaderGraphNodeType>(self) -> Node<T> {
    modify_graph(|graph| self.insert_into_graph(graph))
  }

  pub fn insert_into_graph<T: ShaderGraphNodeType>(
    self,
    builder: &mut ShaderGraphBuilder,
  ) -> Node<T> {
    builder.check_register_type::<T>();

    self.insert_into_graph_inner(builder.top_scope_mut(), T::TYPE)
  }

  fn insert_into_graph_inner<T: ShaderGraphNodeType>(
    self,
    top: &mut ShaderGraphScope,
    ty: ShaderValueType,
  ) -> Node<T> {
    let mut nodes_to_connect = Vec::new();
    self.visit_dependency(|dep| {
      nodes_to_connect.push(*dep);
    });

    let is_write = matches!(self, ShaderGraphNode::Write { .. });

    let result = top
      .insert_node(ShaderGraphNodeData { node: self, ty })
      .handle();

    nodes_to_connect.iter().for_each(|n| {
      if n.graph_id != top.graph_guid {
        top.captured.push(*n);
      } else {
        top.nodes.connect_node(n.handle, result.handle);
      }
    });

    if is_write {
      for barrier in &top.barriers {
        top.nodes.connect_node(barrier.handle, result.handle);
      }
    }

    Node {
      phantom: PhantomData,
      handle: result,
    }
  }

  pub fn visit_dependency(&self, mut visitor: impl FnMut(&ShaderGraphNodeRawHandle)) {
    match self {
      ShaderGraphNode::Expr(expr) => match expr {
        ShaderGraphNodeExpr::FunctionCall { parameters, .. } => parameters.iter().for_each(visitor),
        ShaderGraphNodeExpr::TextureSampling {
          texture,
          sampler,
          position,
          index,
          level,
        } => {
          visitor(texture);
          visitor(sampler);
          visitor(position);
          index.as_ref().map(&mut visitor);
          level.as_ref().map(&mut visitor);
        }
        ShaderGraphNodeExpr::MatShrink { source, .. } => visitor(source),
        ShaderGraphNodeExpr::Swizzle { source, .. } => visitor(source),
        ShaderGraphNodeExpr::Compose { parameters, .. } => parameters.iter().for_each(visitor),
        ShaderGraphNodeExpr::Operator(op) => match op {
          OperatorNode::Unary { one, .. } => visitor(one),
          OperatorNode::Binary { left, right, .. } => {
            visitor(left);
            visitor(right);
          }
          OperatorNode::Index { array, entry, .. } => {
            visitor(array);
            visitor(entry);
          }
        },
        ShaderGraphNodeExpr::FieldGet { struct_node, .. } => visitor(struct_node),
        ShaderGraphNodeExpr::StructConstruct { fields, .. } => fields.iter().for_each(visitor),
        ShaderGraphNodeExpr::Const(_) => {}
        ShaderGraphNodeExpr::Copy(from) => visitor(from),
      },
      ShaderGraphNode::Input(_) => {}
      ShaderGraphNode::UnNamed => {}
      ShaderGraphNode::Write {
        new: source,
        old: target,
        ..
      } => {
        visitor(source);
        target.as_ref().map(visitor);
      }
      ShaderGraphNode::ControlFlow(cf) => match cf {
        ShaderControlFlowNode::If { condition, .. } => visitor(condition),
        ShaderControlFlowNode::For {
          source,
          iter,
          index,
          ..
        } => {
          visitor(iter);
          visitor(index);

          fn visit_iter(
            source: &ShaderIterator,
            visitor: &mut impl FnMut(&ShaderGraphNodeRawHandle),
          ) {
            match source {
              ShaderIterator::Const(_) => {}
              ShaderIterator::Count(c) => visitor(c),
              ShaderIterator::FixedArray { array, .. } => visitor(array),
              ShaderIterator::Clamped { source, max } => {
                visit_iter(source, visitor);
                visitor(max)
              }
            }
          }

          visit_iter(source, &mut visitor);
        }
      },
      ShaderGraphNode::SideEffect(_) => {}
    }
  }

  pub fn replace_dependency(
    &mut self,
    old: ShaderGraphNodeRawHandle,
    new: ShaderGraphNodeRawHandle,
  ) {
    self.visit_dependency_mut(|dep| {
      if *dep == old {
        *dep = new;
      }
    })
  }

  pub fn visit_dependency_mut(&mut self, mut visitor: impl FnMut(&mut ShaderGraphNodeRawHandle)) {
    match self {
      ShaderGraphNode::Expr(expr) => match expr {
        ShaderGraphNodeExpr::FunctionCall { parameters, .. } => {
          parameters.iter_mut().for_each(visitor)
        }
        ShaderGraphNodeExpr::TextureSampling {
          texture,
          sampler,
          position,
          index,
          level,
        } => {
          visitor(texture);
          visitor(sampler);
          visitor(position);
          index.as_mut().map(&mut visitor);
          level.as_mut().map(&mut visitor);
        }
        ShaderGraphNodeExpr::MatShrink { source, .. } => visitor(source),
        ShaderGraphNodeExpr::Swizzle { source, .. } => visitor(source),
        ShaderGraphNodeExpr::Compose { parameters, .. } => parameters.iter_mut().for_each(visitor),
        ShaderGraphNodeExpr::Operator(op) => match op {
          OperatorNode::Unary { one, .. } => visitor(one),
          OperatorNode::Binary { left, right, .. } => {
            visitor(left);
            visitor(right);
          }
          OperatorNode::Index { array, entry, .. } => {
            visitor(array);
            visitor(entry);
          }
        },
        ShaderGraphNodeExpr::FieldGet { struct_node, .. } => visitor(struct_node),
        ShaderGraphNodeExpr::StructConstruct { fields, .. } => fields.iter_mut().for_each(visitor),
        ShaderGraphNodeExpr::Const(_) => {}
        ShaderGraphNodeExpr::Copy(from) => visitor(from),
      },
      ShaderGraphNode::Input(_) => {}
      ShaderGraphNode::UnNamed => {}
      ShaderGraphNode::Write {
        new: source,
        old: target,
        ..
      } => {
        visitor(source);
        target.as_mut().map(visitor);
      }
      ShaderGraphNode::ControlFlow(cf) => match cf {
        ShaderControlFlowNode::If { condition, .. } => visitor(condition),
        ShaderControlFlowNode::For {
          source,
          iter,
          index,
          ..
        } => {
          visitor(iter);
          visitor(index);

          fn visit_iter(
            source: &mut ShaderIterator,
            visitor: &mut impl FnMut(&mut ShaderGraphNodeRawHandle),
          ) {
            match source {
              ShaderIterator::Const(_) => {}
              ShaderIterator::Count(c) => visitor(c),
              ShaderIterator::FixedArray { array, .. } => visitor(array),
              ShaderIterator::Clamped { source, max } => {
                visit_iter(source, visitor);
                visitor(max)
              }
            }
          }

          visit_iter(source, &mut visitor);
        }
      },
      ShaderGraphNode::SideEffect(_) => {}
    }
  }
}
