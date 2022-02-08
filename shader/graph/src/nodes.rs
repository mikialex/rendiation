use crate::*;
use rendiation_algebra::Vec2;
use std::{any::TypeId, marker::PhantomData};

pub trait ShaderGraphNodeType: 'static + Copy {
  fn to_type() -> ShaderValueType;
  fn extract_struct_define() -> Option<&'static ShaderStructMetaInfo> {
    match Self::to_type() {
      ShaderValueType::Fixed(v) => {
        if let ShaderStructMemberValueType::Struct(s) = v {
          Some(s)
        } else {
          None
        }
      }
      _ => None,
    }
  }
}

#[derive(Clone, Copy)]
pub enum ShaderValueType {
  Fixed(ShaderStructMemberValueType),
  Sampler,
  Texture,
  Never,
}

#[derive(Clone, Copy)]
pub enum ShaderStructMemberValueType {
  Primitive(PrimitiveShaderValueType),
  Struct(&'static ShaderStructMetaInfo),
  // FixedSizeArray((&'static ShaderValueType, usize)),
}
pub trait ShaderStructMemberValueNodeType {
  fn to_type() -> ShaderStructMemberValueType;
}

pub trait PrimitiveShaderGraphNodeType: ShaderGraphNodeType {
  fn to_primitive_type() -> PrimitiveShaderValueType;
  fn to_primitive(&self) -> PrimitiveShaderValue;
}

impl<T: PrimitiveShaderGraphNodeType> ShaderGraphNodeType for T {
  fn to_type() -> ShaderValueType {
    ShaderValueType::Fixed(ShaderStructMemberValueType::Primitive(
      T::to_primitive_type(),
    ))
  }
}

impl<T: PrimitiveShaderGraphNodeType> ShaderStructMemberValueNodeType for T {
  fn to_type() -> ShaderStructMemberValueType {
    ShaderStructMemberValueType::Primitive(T::to_primitive_type())
  }
}

pub trait ShaderGraphStructuralNodeType: ShaderGraphNodeType {
  type Instance;
  fn meta_info() -> &'static ShaderStructMetaInfo;
  fn expand(node: Node<Self>) -> Self::Instance;
}

impl<T> From<T> for Node<T>
where
  T: PrimitiveShaderGraphNodeType,
{
  fn from(input: T) -> Self {
    ShaderGraphNodeData::Const(ConstNode {
      data: input.to_primitive(),
    })
    .insert_graph()
  }
}

// this for not include samplers/textures as attributes
pub trait ShaderGraphAttributeNodeType: ShaderGraphNodeType {}

#[derive(Copy, Clone)]
pub struct AnyType;

impl<T> Node<T> {
  /// cast the underlayer handle to untyped, this cast is safe because
  /// we consider this a kind of up casting. Use this will reduce the
  /// unsafe code when we create ShaderGraphNodeData
  pub fn cast_untyped(&self) -> ShaderGraphNodeRawHandleUntyped {
    unsafe { self.handle.get().cast_type() }
  }

  pub fn cast_untyped_node(&self) -> NodeUntyped {
    self.cast_untyped().into()
  }
}

pub struct ShaderGraphNode<T> {
  phantom: PhantomData<T>,
  pub data: ShaderGraphNodeData,
}

impl<T: ShaderGraphNodeType> ShaderGraphNode<T> {
  #[must_use]
  pub fn new(data: ShaderGraphNodeData) -> Self {
    Self {
      data,
      phantom: PhantomData,
    }
  }

  #[must_use]
  pub fn into_any(self) -> ShaderGraphNodeUntyped {
    unsafe { std::mem::transmute(self) }
  }

  #[must_use]
  pub fn into_typed(self) -> ShaderGraphNode<T> {
    unsafe { std::mem::transmute(self) }
  }

  pub fn unwrap_as_input(&self) -> &ShaderGraphInputNode {
    match &self.data {
      ShaderGraphNodeData::Input(n) => n,
      _ => panic!("unwrap as input failed"),
    }
  }
}

pub enum ShaderGraphNodeData {
  FunctionCall(FunctionNode),
  TextureSampling(TextureSamplingNode),
  Swizzle {
    ty: &'static str,
    source: ShaderGraphNodeRawHandleUntyped,
  },
  Compose {
    target: PrimitiveShaderValueType,
    parameters: Vec<ShaderGraphNodeRawHandleUntyped>,
  },
  Operator(OperatorNode),
  Input(ShaderGraphInputNode),
  /// This is workaround for some case
  UnNamed,
  FieldGet {
    field_name: &'static str,
    struct_node: ShaderGraphNodeRawHandleUntyped,
  },
  StructConstruct {
    struct_id: TypeId,
    fields: Vec<ShaderGraphNodeRawHandleUntyped>,
  },
  Copy(ShaderGraphNodeRawHandleUntyped),
  Write {
    source: ShaderGraphNodeRawHandleUntyped,
    target: ShaderGraphNodeRawHandleUntyped,
    implicit: bool,
  },
  Const(ConstNode),
  ControlFlow(ShaderControlFlowNode),
  SideEffect(ShaderSideEffectNode),
}

pub enum ShaderSideEffectNode {
  Continue,
  Break,
  Return(ShaderGraphNodeRawHandleUntyped),
  Termination,
}

pub enum ShaderControlFlowNode {
  If {
    condition: ShaderGraphNodeRawHandleUntyped,
    scope: ShaderGraphScope,
  },
  For {
    source: ShaderIteratorAble,
    scope: ShaderGraphScope,
  },
  // While,
}

#[derive(Clone)]
pub enum ShaderIteratorAble {
  Const(u32),
  Count(Node<u32>),
}

#[derive(Clone)]
pub struct ConstNode {
  pub data: PrimitiveShaderValue,
}

impl ShaderSideEffectNode {
  pub fn insert_graph_bottom(self) {
    self.insert_graph(0);
  }
  pub fn insert_graph(self, target_scope_id: usize) {
    modify_graph(|graph| {
      let node = ShaderGraphNodeData::SideEffect(self).insert_into_graph::<AnyType>(graph);
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
  pub fn collect_captured(&self) -> Vec<ShaderGraphNodeRawHandleUntyped> {
    match self {
      ShaderControlFlowNode::If { scope, .. } => scope.captured.clone(),
      ShaderControlFlowNode::For { scope, .. } => scope.captured.clone(),
    }
  }
  pub fn collect_writes(
    &self,
  ) -> Vec<(
    Rc<Cell<ShaderGraphNodeRawHandleUntyped>>,
    ShaderGraphNodeRawHandleUntyped,
  )> {
    match self {
      ShaderControlFlowNode::If { scope, .. } => scope.writes.clone(),
      ShaderControlFlowNode::For { scope, .. } => scope.writes.clone(),
    }
  }
  pub fn insert_into_graph(self, builder: &mut ShaderGraphBuilder) {
    let has_side_effect = self.has_side_effect();
    let captured = self.collect_captured();
    let writes = self.collect_writes();
    let node = ShaderGraphNodeData::ControlFlow(self).insert_into_graph::<AnyType>(builder);
    let top = builder.top_scope_mut();
    let nodes = &mut top.nodes;

    if has_side_effect {
      top
        .inserted
        .iter()
        .take(top.inserted.len() - 1)
        .for_each(|n| nodes.connect_node(n.handle, node.handle().handle));
      top.barriers.push(node.handle());
    }

    // visit all the node in this scope generate before, and check
    // if it's same and generate dep, if not pass the captured to parent scope
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

    for write in &writes {
      let im_write = ShaderGraphNodeData::Write {
        target: write.1,
        source: node.handle(),
        implicit: true,
      }
      .insert_into_graph_inner::<AnyType>(top);

      write.0.set(im_write.handle());
    }

    for write in writes {
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

impl ShaderGraphNodeData {
  pub fn insert_graph<T: ShaderGraphNodeType>(self) -> Node<T> {
    modify_graph(|graph| self.insert_into_graph(graph))
  }

  pub fn insert_into_graph<T: ShaderGraphNodeType>(
    self,
    builder: &mut ShaderGraphBuilder,
  ) -> Node<T> {
    if let Some(s) = T::extract_struct_define() {
      builder.struct_defines.insert(TypeId::of::<T>(), s);
    }

    self.insert_into_graph_inner(builder.top_scope_mut())
  }

  pub fn insert_into_graph_inner<T: ShaderGraphNodeType>(
    self,
    top: &mut ShaderGraphScope,
  ) -> Node<T> {
    let mut nodes_to_connect = Vec::new();
    self.visit_dependency(|dep| {
      nodes_to_connect.push(*dep);
    });

    let node = ShaderGraphNode::<T>::new(self);
    let result = top.insert_node(node).handle();

    nodes_to_connect.iter().for_each(|n| {
      if n.graph_id != top.graph_guid {
        top.captured.push(*n);
      } else {
        top.nodes.connect_node(n.handle, result.handle);
      }
    });

    for barrier in &top.barriers {
      top.nodes.connect_node(barrier.handle, result.handle);
    }

    unsafe { result.cast_type().into() }
  }
  pub fn visit_dependency(&self, mut visitor: impl FnMut(&ShaderGraphNodeRawHandleUntyped)) {
    match self {
      ShaderGraphNodeData::FunctionCall(FunctionNode { parameters, .. }) => {
        parameters.iter().for_each(visitor)
      }
      ShaderGraphNodeData::TextureSampling(TextureSamplingNode {
        texture,
        sampler,
        position,
      }) => unsafe {
        visitor(&texture.cast_type());
        visitor(&sampler.cast_type());
        visitor(&position.cast_type());
      },
      ShaderGraphNodeData::Swizzle { source, .. } => visitor(source),
      ShaderGraphNodeData::Compose { parameters, .. } => parameters.iter().for_each(visitor),
      ShaderGraphNodeData::Operator(OperatorNode { left, right, .. }) => {
        visitor(left);
        visitor(right);
      }
      ShaderGraphNodeData::Input(_) => {}
      ShaderGraphNodeData::FieldGet { struct_node, .. } => visitor(struct_node),
      ShaderGraphNodeData::StructConstruct { fields, .. } => fields.iter().for_each(visitor),
      ShaderGraphNodeData::Const(_) => {}
      ShaderGraphNodeData::UnNamed => {}
      ShaderGraphNodeData::Copy(from) => visitor(from),
      ShaderGraphNodeData::Write { source, target, .. } => {
        visitor(source);
        visitor(target);
      }
      ShaderGraphNodeData::ControlFlow(cf) => match cf {
        ShaderControlFlowNode::If { condition, .. } => visitor(condition),
        ShaderControlFlowNode::For { source, .. } => {
          // todo
        }
      },
      ShaderGraphNodeData::SideEffect(_) => {}
    }
  }
}

#[derive(Clone)]
pub struct FunctionNode {
  pub prototype: &'static ShaderFunctionMetaInfo,
  pub parameters: Vec<ShaderGraphNodeRawHandleUntyped>,
}

#[derive(Clone)]
pub struct TextureSamplingNode {
  pub texture: ShaderGraphNodeRawHandle<ShaderTexture>,
  pub sampler: ShaderGraphNodeRawHandle<ShaderSampler>,
  pub position: ShaderGraphNodeRawHandle<Vec2<f32>>,
}

#[derive(Clone)]
pub struct OperatorNode {
  pub left: ShaderGraphNodeRawHandleUntyped,
  pub right: ShaderGraphNodeRawHandleUntyped,
  pub operator: &'static str,
}

pub enum UnaryOperator {
  Not,
}

pub enum BinaryOperator {
  Add,
  Sub,
  Mul,
  Div,
  Eq,
  NotEq,
  GreaterThan,
  LessThan,
  GreaterEqualThan,
  LessEqualThan,
}

pub enum TrinaryOperator {
  IfElse,
}

pub enum OperatorNode2 {
  Unary {
    one: ShaderGraphNodeRawHandleUntyped,
    operator: &'static str,
  },
  Binary {
    left: ShaderGraphNodeRawHandleUntyped,
    right: ShaderGraphNodeRawHandleUntyped,
    operator: &'static str,
  },
  Trinary {
    forward: ShaderGraphNodeRawHandleUntyped,
    left: ShaderGraphNodeRawHandleUntyped,
    right: ShaderGraphNodeRawHandleUntyped,
    operator: &'static str,
  },
}

#[derive(Clone)]
pub enum ShaderGraphInputNode {
  BuiltIn(ShaderBuiltIn),
  Uniform {
    bindgroup_index: usize,
    entry_index: usize,
  },
  VertexIn {
    ty: PrimitiveShaderValueType,
    index: usize,
  },
  FragmentIn {
    ty: PrimitiveShaderValueType,
    index: usize,
  },
}

#[derive(Copy, Clone)]
pub enum ShaderBuiltIn {
  VertexClipPosition,
  VertexPointSize,
  VertexIndexId,
  VertexInstanceId,
}

// todo
#[derive(Copy, Clone)]
pub enum ShaderGraphVertexFragmentIOType {
  Float,
}
