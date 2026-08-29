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

/// Declare a shader semantic type usable only in the vertex stage.
#[macro_export]
macro_rules! only_vertex {
  ($(#[$attr:meta])* $Type: ident, $NodeType: ty) => {
    #[doc = concat!(stringify!($Type), " is a vertex stage only shader semantic, its value type is ", stringify!($NodeType), ".\n\n")]
    $(#[$attr])*
    pub struct $Type;
    impl SemanticVertexShaderValue for $Type {
      type ValueType = $NodeType;
    }
  };
}

/// Declare a shader semantic type usable only in the fragment stage.
#[macro_export]
macro_rules! only_fragment {
  ($(#[$attr:meta])* $Type: ident, $NodeType: ty) => {
    #[doc = concat!(stringify!($Type), " is a fragment stage only shader semantic, its value type is ", stringify!($NodeType), ".\n\n")]
    $(#[$attr])*
    pub struct $Type;
    impl SemanticFragmentShaderValue for $Type {
      type ValueType = $NodeType;
    }
  };
}

/// Declare a shader semantic type usable in both the vertex and fragment stages.
#[macro_export]
macro_rules! both {
  ($(#[$attr:meta])* $Type: ident, $NodeType: ty) => {
    #[doc = concat!(stringify!($Type), " is a shader semantic shared by the vertex and fragment stages, its value type is ", stringify!($NodeType), ".\n\n")]
    $(#[$attr])*
    pub struct $Type;
    impl SemanticVertexShaderValue for $Type {
      type ValueType = $NodeType;
    }
    impl SemanticFragmentShaderValue for $Type {
      type ValueType = $NodeType;
    }
  };
}

pub const ENABLE_DEFAULT_DISPLAY_DEBUG: bool = false;
thread_local! {
  pub static DEFAULT_DISPLAY_DEBUG: RefCell<Option<ShaderPtrOf<Vec3<f32>>>> = const { RefCell::new(None) };
}

both!(DefaultDisplay, Vec4<f32>);

only_vertex!(
  /// WGSL built-in `vertex_index` (see <https://www.w3.org/TR/WGSL/#builtin-value-names>).
  ///
  /// Index of the current vertex within the current API-level draw command,
  /// independent of draw instancing.
  ///
  /// For a non-indexed draw, the first vertex has an index equal to the
  /// `firstVertex` argument of the draw, whether provided directly or
  /// indirectly. The index is incremented by one for each additional vertex in
  /// the draw instance.
  ///
  /// For an indexed draw, the index is equal to the index buffer entry for the
  /// vertex, plus the `baseVertex` argument of the draw, whether provided
  /// directly or indirectly.
  VertexIndex,
  u32
);

only_vertex!(
  /// WGSL built-in `instance_index` (see <https://www.w3.org/TR/WGSL/#builtin-value-names>).
  ///
  /// Instance index of the current vertex within the current API-level draw
  /// command.
  VertexInstanceIndex,
  u32
);

only_vertex!(
  /// WGSL built-in `position` in the vertex stage (see <https://www.w3.org/TR/WGSL/#builtin-value-names>).
  ///
  /// The clip position of the current vertex, in clip space coordinates.
  ///
  /// An output value (x, y, z, w) will map to (x/w, y/w, z/w) in WebGPU
  /// normalized device coordinates.
  ClipPosition,
  Vec4<f32>
);

only_fragment!(
  /// WGSL built-in `front_facing` (see <https://www.w3.org/TR/WGSL/#builtin-value-names>).
  ///
  /// True when the current fragment is on a front-facing primitive.
  /// False otherwise.
  FragmentFrontFacing,
  bool
);

only_fragment!(
  /// WGSL built-in `position` in the fragment stage (see <https://www.w3.org/TR/WGSL/#builtin-value-names>).
  ///
  /// Input position of the current fragment rasterization point. <https://gpuweb.github.io/gpuweb/#rasterizationpoint>
  ///
  /// In more detail:
  /// * `fp.x` and `fp.y` are the interpolated x and y coordinates of the
  ///   position of the current fragment rasterization point in the framebuffer.
  ///   The framebuffer is a two-dimensional grid of pixels with the top-left at
  ///   (0.0, 0.0) and the bottom right at (`vp.width`, `vp.height`). Each pixel
  ///   has an extent of 1.0 unit in each of the x and y dimensions, and pixel
  ///   centers are at (0.5, 0.5) offset from integer coordinates.
  /// * `fp.z` is the interpolated depth of the current fragment rasterization
  ///   point. For example, depth 0 in normalized device coordinates maps to
  ///   `fp.z = vp.minDepth`, and depth 1 in normalized device coordinates maps
  ///   to `fp.z = vp.maxDepth`.
  /// * `fp.w` is the perspective divisor for the fragment rasterization point,
  ///   which is the interpolation of `1.0 / vertex_w`, where `vertex_w` is the
  ///   w component of the `position` output of the vertex shader.
  FragmentPosition,
  Vec4<f32>
);

only_fragment!(
  /// WGSL built-in `sample_index` (see <https://www.w3.org/TR/WGSL/#builtin-value-names>).
  ///
  /// Sample index for the current fragment rasterization point. The value is
  /// at least 0 and at most `sampleCount - 1`, where `sampleCount` is the
  /// sample count specified for the GPU render pipeline.
  ///
  /// When this attribute is applied, if the effects of the fragment shader
  /// would vary based on the value of `sample_index`, the fragment shader will
  /// be invoked once per sample.
  FragmentSampleIndex,
  u32
);

only_fragment!(
  /// WGSL built-in `sample_mask` in the fragment stage, input direction (see <https://www.w3.org/TR/WGSL/#builtin-value-names>).
  ///
  /// Sample coverage bitmask for the current fragment.
  ///
  /// Bits are indexed by a sample index in the half-open interval
  /// [0, `sampleCount`), where `sampleCount` is the sample count specified for
  /// the GPU render pipeline.
  ///
  /// A bit will be set to 1 only if the sample is covered by the primitive
  /// being rendered. Bits with index `sampleCount` and above are always set to
  /// zero.
  ///
  /// There are two possible values for the bitmask:
  /// * The bitmask has a 1 bit set for every sample covered by the fragment.
  ///   In this case the bitmask equals the fragment's coverage mask.
  /// * The bitmask has only 1 bit set, corresponding to the sample being
  ///   processed by the current fragment shader invocation. That is, the
  ///   bitmask is `1 << sample_index`, as if using the `sample_index` built-in
  ///   input.
  ///
  /// Note: These two cases are the same when `sampleCount = 1`.
  ///
  /// Note: This is a known portability hazard when `sampleCount > 1`. Some
  /// devices yield the full coverage mask. Other devices yield the single-bit
  /// mask.
  FragmentSampleMaskInput,
  u32
);

// fragment output
only_fragment!(
  /// WGSL built-in `frag_depth` (see <https://www.w3.org/TR/WGSL/#builtin-value-names>).
  ///
  /// Updated depth of the fragment, in the viewport depth range.
  ///
  /// If the `fragment_depth` feature is supported, then the builtin attribute
  /// can have an optional second parameter to specify a depth mode:
  /// * Specifying the depth mode `less` is a promise that if the fragment
  ///   shader returns `frag_depth` (i.e. doesn't `discard`), then the returned
  ///   value will be less than or equal to the original depth of the fragment.
  /// * Specifying the depth mode `greater` is a promise that if the fragment
  ///   shader returns `frag_depth` (i.e. doesn't `discard`), then the returned
  ///   value will be greater than or equal to the original depth of the
  ///   fragment.
  ///
  /// The original depth of the fragment is the depth property of the
  /// fragment's rasterization point.
  ///
  /// If the shader returns a depth value that violates the depth mode promise,
  /// then an indeterminate depth value may be used instead.
  FragmentDepthOutput,
  f32
);

only_fragment!(
  /// WGSL built-in `sample_mask` in the fragment stage, output direction (see <https://www.w3.org/TR/WGSL/#builtin-value-names>).
  ///
  /// Sample coverage mask control for the current fragment. The last value
  /// written to this variable becomes the shader-output mask.
  ///
  /// Zero bits in the written value will cause corresponding samples in the
  /// color attachments to be discarded.
  FragmentSampleMaskOutput,
  u32
);
