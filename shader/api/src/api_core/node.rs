use crate::*;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Node<T> {
  phantom: PhantomData<T>,
  handle: ShaderNodeRawHandle,
}

impl<T> Node<T> {
  pub fn handle(&self) -> ShaderNodeRawHandle {
    self.handle
  }

  /// # Safety
  /// force type casting
  pub unsafe fn cast_type<X>(self) -> Node<X>
  where
    X: ShaderNodeType,
  {
    std::mem::transmute(self)
  }
}

impl<T> From<T> for Node<T>
where
  T: PrimitiveShaderNodeType,
{
  fn from(input: T) -> Self {
    ShaderNodeExpr::Const {
      data: input.to_primitive(),
    }
    .insert_api()
  }
}

// todo restrict type to referable?
impl<T: ShaderNodeType> Node<T> {
  pub fn make_local_var(&self) -> LocalVarNode<T> {
    call_shader_api(|g| unsafe {
      let v = g.make_local_var(T::TYPE);
      g.store(self.handle(), v);
      v.into_node()
    })
  }
}

impl<T, const W: AddressSpace> Node<ShaderPtr<T, W>> {
  pub fn load_unchecked(&self) -> Node<T> {
    call_shader_api(|g| unsafe { g.load(self.handle()).into_node() })
  }

  pub fn load(&self) -> Node<T>
  where
    TruthCheckBool<{ W.loadable() }>: TruthCheckPass,
  {
    call_shader_api(|g| unsafe { g.load(self.handle()).into_node() })
  }

  pub fn store_unchecked(&self, source: impl Into<Node<T>>) {
    let source = source.into();
    call_shader_api(|g| {
      g.store(source.handle(), self.handle());
    })
  }

  pub fn store(&self, source: impl Into<Node<T>>)
  where
    TruthCheckBool<{ W.writeable() }>: TruthCheckPass,
  {
    let source = source.into();
    call_shader_api(|g| {
      g.store(source.handle(), self.handle());
    })
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
pub struct ShaderNodeRawHandle {
  pub handle: usize,
}

impl ShaderNodeRawHandle {
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
