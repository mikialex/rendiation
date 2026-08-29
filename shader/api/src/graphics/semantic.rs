use anymap::ClonableAnyMap;

use crate::*;

#[derive(Default, Clone)]
pub struct SemanticRegistry {
  pub static_semantic: FastHashMap<TypeId, NodeUntyped>,
  pub dynamic_tag: FastHashSet<TypeId>,
  pub any_map: ClonableAnyMap,
}

impl SemanticRegistry {
  pub fn contains_type_tag<T: Any>(&self) -> bool {
    self.dynamic_tag.contains(&TypeId::of::<T>())
  }
  pub fn insert_type_tag<T: Any>(&mut self) {
    self.dynamic_tag.insert(TypeId::of::<T>());
  }

  #[track_caller]
  pub fn try_query_typed_both_stage<T: SemanticFragmentShaderValue + SemanticVertexShaderValue>(
    &self,
  ) -> Result<Node<<T as SemanticFragmentShaderValue>::ValueType>, ShaderBuildError> {
    self
      .try_query_raw(TypeId::of::<T>(), <T as SemanticFragmentShaderValue>::NAME)
      .map(|n| unsafe { n.cast_type() })
  }

  #[track_caller]
  pub fn try_query_fragment_stage<T: SemanticFragmentShaderValue>(
    &self,
  ) -> Result<Node<T::ValueType>, ShaderBuildError> {
    self
      .try_query_raw(TypeId::of::<T>(), T::NAME)
      .map(|n| unsafe { n.cast_type() })
  }

  #[track_caller]
  pub fn try_query_vertex_stage<T: SemanticVertexShaderValue>(
    &self,
  ) -> Result<Node<T::ValueType>, ShaderBuildError> {
    self
      .try_query_raw(TypeId::of::<T>(), T::NAME)
      .map(|n| unsafe { n.cast_type() })
  }

  pub fn register_typed_both_stage<T: SemanticVertexShaderValue + SemanticFragmentShaderValue>(
    &mut self,
    node: impl Into<Node<<T as SemanticVertexShaderValue>::ValueType>>,
  ) {
    let node = node.into().cast_untyped_node();
    node.mark_debug_label(get_name::<T>());
    self.register_raw(TypeId::of::<T>(), node);
  }

  pub fn register_vertex_stage<T: SemanticVertexShaderValue>(
    &mut self,
    node: impl Into<Node<T::ValueType>>,
  ) {
    let node = node.into().cast_untyped_node();
    node.mark_debug_label(get_name::<T>());
    self.register_raw(TypeId::of::<T>(), node);
  }

  pub fn register_fragment_stage<T: SemanticFragmentShaderValue>(
    &mut self,
    node: impl Into<Node<T::ValueType>>,
  ) {
    let node = node.into().cast_untyped_node();
    node.mark_debug_label(get_name::<T>());
    self.register_raw(TypeId::of::<T>(), node);
  }

  #[track_caller]
  pub fn try_query_raw(
    &self,
    id: TypeId,
    name: &'static str,
  ) -> Result<NodeUntyped, ShaderBuildError> {
    self
      .static_semantic
      .get(&id)
      .copied()
      .ok_or(ShaderBuildError::MissingRequiredDependency(
        name,
        *Location::caller(),
      ))
  }

  pub fn register_raw(&mut self, id: TypeId, node: NodeUntyped) {
    self.static_semantic.insert(id, node);
  }
}

pub(crate) fn get_name<T: Any>() -> String {
  let name = disqualified::ShortName(std::any::type_name::<T>());
  let name = name.to_string();
  camel_to_snake(&name)
}

fn camel_to_snake(s: &str) -> String {
  let mut snake = String::with_capacity(s.len() + s.len() / 2);

  for (i, ch) in s.chars().enumerate() {
    if ch.is_uppercase() {
      if i > 0 {
        snake.push('_');
      }
      snake.push(ch.to_ascii_lowercase());
    } else {
      snake.push(ch);
    }
  }
  snake
}

#[macro_export]
macro_rules! only_vertex {
  ($Type: ident, $NodeType: ty) => {
    pub struct $Type;
    impl SemanticVertexShaderValue for $Type {
      type ValueType = $NodeType;
    }
  };
}

#[macro_export]
macro_rules! only_fragment {
  ($Type: ident, $NodeType: ty) => {
    pub struct $Type;
    impl SemanticFragmentShaderValue for $Type {
      type ValueType = $NodeType;
    }
  };
}

#[macro_export]
macro_rules! both {
  ($Type: ident, $NodeType: ty) => {
    pub struct $Type;
    impl SemanticVertexShaderValue for $Type {
      type ValueType = $NodeType;
    }
    impl SemanticFragmentShaderValue for $Type {
      type ValueType = $NodeType;
    }
  };
}

//////
// wgsl builtin https://www.w3.org/TR/WGSL/#builtin-values

pub const ENABLE_DEFAULT_DISPLAY_DEBUG: bool = false;
thread_local! {
  pub static DEFAULT_DISPLAY_DEBUG: RefCell<Option<ShaderPtrOf<Vec3<f32>>>> = const { RefCell::new(None) };
}

both!(DefaultDisplay, Vec4<f32>);

// vertex input
only_vertex!(VertexIndex, u32);
only_vertex!(VertexInstanceIndex, u32);

// vertex output
only_vertex!(ClipPosition, Vec4<f32>);

// fragment input
both!(FragmentFrontFacing, bool);
// https://gpuweb.github.io/gpuweb/#rasterizationpoint
// for xy, it's in framebuffer coordinates
both!(FragmentPosition, Vec4<f32>);
both!(FragmentSampleIndex, u32);
both!(FragmentSampleMaskInput, u32);

// fragment output
both!(FragmentDepthOutput, f32);
both!(FragmentSampleMaskOutput, u32);
