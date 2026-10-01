use naga::MemoryDecorations;

use crate::*;

/// What the shader node maps to.
#[derive(Clone, Copy)]
pub(crate) enum NodeSlot {
  Fake,
  /// the expression in the arena of the function where the node is created, it can only be used
  /// in that function
  Expr {
    fn_id: u32,
    expr: naga::Handle<naga::Expression>,
  },
  /// materialized as Expression::GlobalVariable in each function that uses it
  Global(naga::Handle<naga::GlobalVariable>),
  /// materialized as Expression::Constant in each function that uses it, inlined for compose
  Constant(naga::Handle<naga::Constant>),
}

impl ShaderAPINagaImpl {
  pub(crate) fn new_node(&mut self, slot: NodeSlot) -> ShaderNodeRawHandle {
    let node = ShaderNodeRawHandle {
      handle: self.nodes.len(),
    };
    self.nodes.push(slot);
    node
  }

  /// Create a new node that maps to the expression in the building function.
  pub(crate) fn map_new_node(
    &mut self,
    expr: naga::Handle<naga::Expression>,
  ) -> ShaderNodeRawHandle {
    let fn_id = self.building_fn().id;
    self.new_node(NodeSlot::Expr { fn_id, expr })
  }

  /// The WGSL output refers the mesh output variable and the task payload by name in the entry point
  /// attributes, so they must be named.
  pub(crate) fn name_global_if_unnamed(
    &mut self,
    global: naga::Handle<naga::GlobalVariable>,
    name: &str,
  ) {
    let var = self.module.global_variables.get_mut(global);
    if var.name.is_none() {
      var.name = Some(name.to_owned());
    }
  }

  pub(crate) fn declare_global(
    &mut self,
    space: naga::AddressSpace,
    binding: Option<naga::ResourceBinding>,
    ty: naga::Handle<naga::Type>,
  ) -> ShaderNodeRawHandle {
    let global = naga::GlobalVariable {
      name: None,
      space,
      binding,
      ty,
      init: None,
      memory_decorations: MemoryDecorations::empty(),
    };
    let global = self.module.global_variables.append(global, Span::UNDEFINED);
    // the expression is created in the declaring function immediately, so the expression indices
    // of this function do not depend on where the global is first used
    self.building_fn_mut().global_expr(global);
    self.new_node(NodeSlot::Global(global))
  }

  /// The global variable of the node, the node must be declared by [Self::declare_global].
  pub(crate) fn get_global(&self, node: ShaderNodeRawHandle) -> naga::Handle<naga::GlobalVariable> {
    match self.nodes[node.handle] {
      NodeSlot::Global(global) => global,
      _ => panic!("the shader node is not a global variable"),
    }
  }

  /// The expression of the node in the building function. The global variables and the constants
  /// can be used in any function, the other nodes can only be used in the function where they are
  /// created.
  pub(crate) fn get_expression(
    &mut self,
    handle: ShaderNodeRawHandle,
  ) -> naga::Handle<naga::Expression> {
    match self.nodes[handle.handle] {
      NodeSlot::Fake => panic!("the fake shader node can not be used as an expression"),
      NodeSlot::Expr { fn_id, expr } => {
        assert!(
          fn_id == self.building_fn().id,
          "the shader node is used outside of the function where it is created, \
           pass it to the function as a parameter instead"
        );
        expr
      }
      NodeSlot::Global(global) => self.building_fn_mut().global_expr(global),
      NodeSlot::Constant(constant) => self.building_fn_mut().constant_expr(constant),
    }
  }

  // the label is best effort, the unknown node and the node of another function are skipped
  pub(crate) fn mark_handle_debug_name_impl(&mut self, h: ShaderNodeRawHandle, name: String) {
    let Some(slot) = self.nodes.get(h.handle).copied() else {
      return;
    };
    let Some(top_fn) = self.functions.last_mut() else {
      return;
    };

    let handle = match slot {
      NodeSlot::Fake => return,
      NodeSlot::Global(g) => {
        let var = self.module.global_variables.get_mut(g);
        // avoid override for global var
        if var.name.is_none() {
          var.name = Some(name);
        }
        return;
      }
      // only the constant expression that exists in the building function is named
      NodeSlot::Constant(c) => match top_fn.constant_exprs.get(&c) {
        Some(expr) => *expr,
        None => return,
      },
      NodeSlot::Expr { fn_id, expr } if fn_id == top_fn.id => expr,
      NodeSlot::Expr { .. } => return,
    };

    let top_fn = &mut top_fn.function;
    match top_fn.expressions[handle] {
      naga::Expression::FunctionArgument(idx) => {
        top_fn.arguments[idx as usize].name = Some(name);
      }
      // the local variable expression is never emitted, so the named expression is not used
      naga::Expression::LocalVariable(v) => {
        top_fn.local_variables[v].name = Some(name);
      }
      _ => {
        top_fn.named_expressions.insert(handle, name);
      }
    }
  }
}
