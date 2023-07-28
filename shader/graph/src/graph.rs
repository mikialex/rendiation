use crate::*;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Node<T> {
  pub(crate) phantom: PhantomData<T>,
  pub(crate) handle: ShaderGraphNodeRawHandle,
}

impl<T> Node<T> {
  pub fn handle(&self) -> ShaderGraphNodeRawHandle {
    self.handle
  }

  /// # Safety
  /// force type casting
  pub unsafe fn cast_type<X>(self) -> Node<X>
  where
    X: ShaderGraphNodeType,
  {
    modify_graph(|g| g.check_register_type::<X>());
    std::mem::transmute(self)
  }
}

impl<T> From<T> for Node<T>
where
  T: PrimitiveShaderGraphNodeType,
{
  fn from(input: T) -> Self {
    ShaderGraphNodeExpr::Const(ConstNode {
      data: input.to_primitive(),
    })
    .insert_graph()
  }
}

pub struct PendingResolve {
  pub current: Cell<ShaderGraphNodeRawHandle>,
  pub last_depends_history: RefCell<Vec<ShaderGraphNodeRawHandle>>,
}

#[repr(transparent)]
pub struct NodeMutable<T> {
  pub(crate) phantom: PhantomData<T>,
  pub(crate) pending: Rc<PendingResolve>,
}

impl<T: ShaderGraphNodeType> Node<T> {
  pub fn mutable(&self) -> NodeMutable<T> {
    let node = ShaderGraphNodeExpr::Copy(self.handle()).insert_graph::<T>();
    let pending = modify_graph(|builder| {
      let top = builder.top_scope_mut();
      let pending = PendingResolve {
        current: Cell::new(node.handle()),
        last_depends_history: Default::default(),
      };
      let pending = Rc::new(pending);
      top.unresolved.push(pending.clone());
      pending
    });
    NodeMutable {
      phantom: PhantomData,
      pending,
    }
  }
}

impl<T: ShaderGraphNodeType> NodeMutable<T> {
  pub fn get(&self) -> Node<T> {
    unsafe { self.pending.current.get().into_node() }
  }

  pub fn get_last(&self) -> Node<T> {
    // the reason we should clone node here is that
    // when we finally resolve dependency, we should distinguish between
    // the node we want replace the dependency or not, so this copy will
    // actually not code gen and will be replaced by the last resolve node.
    let node = ShaderGraphNodeExpr::Copy(self.get().handle()).insert_graph();
    self
      .pending
      .last_depends_history
      .borrow_mut()
      .push(node.handle());
    node
  }

  pub fn set(&self, node: impl Into<Node<T>>) {
    let node = node.into();
    let write = modify_graph(|builder| {
      let current = self.pending.current.get();
      if current.graph_id != builder.top_scope().graph_guid {
        builder
          .top_scope_mut()
          .writes
          .push((self.pending.clone(), current));
      }

      ShaderGraphNode::Write {
        new: node.handle(),
        old: self.get().handle().into(),
      }
      .insert_into_graph::<AnyType>(builder)
    });

    self.pending.current.set(write.handle())
  }
}

#[derive(Copy, Clone)]
pub struct AnyType;

impl<T> Node<T> {
  pub fn cast_untyped_node(&self) -> NodeUntyped {
    unsafe { std::mem::transmute_copy(self) }
  }
}

pub type NodeUntyped = Node<AnyType>;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderGraphNodeRawHandle {
  pub handle: ArenaGraphNodeHandle<ShaderGraphNodeData>,
  pub graph_id: usize,
}

impl ShaderGraphNodeRawHandle {
  /// # Safety
  ///
  /// force type casting
  pub unsafe fn into_node<X>(&self) -> Node<X> {
    Node {
      handle: *self,
      phantom: PhantomData,
    }
  }

  pub fn into_node_untyped(&self) -> NodeUntyped {
    unsafe { self.into_node() }
  }
}

pub struct ShaderGraphBuilder {
  pub scopes: Vec<ShaderGraphScope>,
  /// if struct insert order matters, we have to use linked hashmap
  ///
  /// this only contains struct used directly by node api.
  /// the struct used by shader fragments will be considered later
  pub struct_defines: FastHashSet<&'static ShaderStructMetaInfo>,
}

impl Default for ShaderGraphBuilder {
  fn default() -> Self {
    Self {
      scopes: vec![Default::default()],
      struct_defines: Default::default(),
    }
  }
}

// todo unsized struct
fn extract_struct_define(
  ty: &ShaderValueType,
  visitor: &mut impl FnMut(&'static ShaderStructMetaInfo),
) {
  ty.visit_single(|ty| {
    if let ShaderValueSingleType::Fixed(v) = ty {
      extract_struct_define_inner(v, visitor)
    }
  });
}

fn extract_struct_define_inner(
  ty: &ShaderStructMemberValueType,
  visitor: &mut impl FnMut(&'static ShaderStructMetaInfo),
) {
  match ty {
    ShaderStructMemberValueType::Primitive(_) => {}
    ShaderStructMemberValueType::Struct(s) => visitor(s),
    ShaderStructMemberValueType::FixedSizeArray((ty, _)) => {
      extract_struct_define_inner(ty, visitor)
    }
  }
}

impl ShaderGraphBuilder {
  pub fn check_register_type<T: ShaderGraphNodeType>(&mut self) {
    extract_struct_define(&T::TYPE, &mut |s| self.check_insert(s));
  }

  fn check_insert(&mut self, s: &'static ShaderStructMetaInfo) {
    if self.struct_defines.insert(s) {
      for f in s.fields {
        if let ShaderStructMemberValueType::Struct(s) = f.ty {
          self.check_insert(s)
        }
      }
    }
  }

  pub fn top_scope_mut(&mut self) -> &mut ShaderGraphScope {
    self.scopes.last_mut().unwrap()
  }
  pub fn top_scope(&self) -> &ShaderGraphScope {
    self.scopes.last().unwrap()
  }

  pub fn push_scope(&mut self) -> &mut ShaderGraphScope {
    self.scopes.push(Default::default());
    self.top_scope_mut()
  }

  pub fn pop_scope(&mut self) -> ShaderGraphScope {
    let mut scope = self.scopes.pop().unwrap();
    scope.resolve_all_pending();
    scope
  }
}

pub struct ShaderGraphScope {
  pub graph_guid: usize,
  pub has_side_effect: bool,
  pub nodes: ArenaGraph<ShaderGraphNodeData>,
  /// every node write in this scope in sequence
  pub inserted: Vec<ShaderGraphNodeRawHandle>,
  /// every effect node has wrote to this scope
  ///
  /// To keep the control flow order correct:
  /// any effect node write should depend any inserted before,
  /// any node write should depend all written effect nodes ;
  pub barriers: Vec<ShaderGraphNodeRawHandle>,
  /// any scoped inserted nodes's dependency which not exist in current scope.
  /// when scope popped, ths captured node will generate dependency in parent scope
  /// and pop again if the node is not find in parent scope
  pub captured: Vec<ShaderGraphNodeRawHandle>,
  /// any scoped inserted nodes's write to dependency which not exist in current scope.
  /// when scope popped, ditto
  ///
  /// require clone Rc<PendingResolve> is to add the implicit write node after the scope
  pub writes: Vec<(Rc<PendingResolve>, ShaderGraphNodeRawHandle)>,

  pub unresolved: Vec<Rc<PendingResolve>>,
}

static SCOPE_GUID: AtomicUsize = AtomicUsize::new(0);
impl Default for ShaderGraphScope {
  fn default() -> Self {
    Self {
      graph_guid: SCOPE_GUID.fetch_add(1, Ordering::Relaxed),
      has_side_effect: Default::default(),
      nodes: Default::default(),
      inserted: Default::default(),
      barriers: Default::default(),
      captured: Default::default(),
      writes: Default::default(),
      unresolved: Default::default(),
    }
  }
}

impl ShaderGraphScope {
  pub fn resolve_all_pending(&mut self) {
    let nodes = &mut self.nodes;
    self.unresolved.drain(..).for_each(|p| {
      p.last_depends_history.borrow().iter().for_each(|old_h| {
        let old = nodes.get_node(old_h.handle);
        let mut dependency = old.from().clone();
        dependency.drain(..).for_each(|d| {
          let dd = nodes.get_node_mut(d);
          dd.data_mut()
            .node
            .replace_dependency(*old_h, p.current.get());
          nodes.connect_node(p.current.get().handle, d);
          // todo cut old connection;
          // todo check fix duplicate connection
        })
      })
    })
  }

  pub fn insert_node(&mut self, node: ShaderGraphNodeData) -> NodeUntyped {
    let handle = ShaderGraphNodeRawHandle {
      handle: self.nodes.create_node(node),
      graph_id: self.graph_guid,
    };
    self.inserted.push(handle);
    handle.into_node_untyped()
  }
}
