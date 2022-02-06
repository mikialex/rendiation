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

#[derive(Clone)]
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

#[derive(Clone)]
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
  Named(String),
  FieldGet {
    field_name: &'static str,
    struct_node: ShaderGraphNodeRawHandleUntyped,
  },
  StructConstruct {
    struct_id: TypeId,
    fields: Vec<ShaderGraphNodeRawHandleUntyped>,
  },
  Copy(ShaderGraphNodeRawHandleUntyped),
  Const(ConstNode),
  Scope(ShaderScopeNode),
  SideEffect(ShaderSideEffectNode),
}

#[derive(Clone)]
pub enum ShaderSideEffectNode {
  Continue,
  Break,
  Return,
  Termination,
}

#[derive(Clone)]
pub enum ShaderScopeNode {
  If {
    condition: ShaderGraphNodeRawHandleUntyped,
    scope: ArenaGraph<ShaderGraphNodeUntyped>,
  },
  For {},
  // While,
}

#[derive(Clone)]
pub struct ConstNode {
  pub data: PrimitiveShaderValue,
}

impl ShaderGraphNodeData {
  pub fn insert_effect() {
    //
  }

  pub fn insert_graph<T: ShaderGraphNodeType>(self) -> Node<T> {
    modify_graph(|graph| self.insert_into_graph(graph))
  }

  pub fn insert_into_graph<T: ShaderGraphNodeType>(
    self,
    builder: &mut ShaderGraphBuilder,
  ) -> Node<T> {
    // let language = WGSL;

    if let Some(s) = T::extract_struct_define() {
      builder.struct_defines.insert(TypeId::of::<T>(), s);
    }

    let graph = builder.top_scope();
    let node = ShaderGraphNode::<T>::new(self.clone());
    let result = graph.insert_node(node).handle();

    // if let ShaderGraphNodeData::Input(input) = &self {
    //   builder.top_scope().code_gen.code_gen_history.insert(
    //     result,
    //     MiddleVariableCodeGenResult {
    //       var_name: language.gen_input_name(input),
    //       statement: "".to_owned(),
    //     },
    //   );
    // }

    // if let Some((var_name, statement)) = language.gen_statement(&self, builder) {
    //   builder.code_builder.write_ln(&statement);
    //   builder.top_scope().code_gen.code_gen_history.insert(
    //     result,
    //     MiddleVariableCodeGenResult {
    //       var_name,
    //       statement,
    //     },
    //   );
    // }

    self.visit_dependency(|dep| {
      builder
        .top_scope()
        .nodes
        .connect_node(dep.handle, result.handle);
    });
    let graph = builder.top_scope();
    for barrier in &graph.barriers {
      graph.nodes.connect_node(barrier.handle, result.handle);
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
      _ => todo!(),
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
