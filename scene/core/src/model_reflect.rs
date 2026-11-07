//! Reflectable mirrors of the wgpu types used by [RasterizationStates], used as its facet proxy so
//! that the reflection based consumers (for example the C ffi) can see its content.

use wgpu_types::*;

use crate::*;

#[derive(Facet, Debug, Clone, Copy, PartialEq, Default)]
pub struct RasterizationStatesReflect {
  pub depth_write_enabled: bool,
  pub depth_compare: SemanticCompareFunction,
  pub stencil: StencilStateReflect,
  pub bias: DepthBiasStateReflect,
  pub blend: Option<BlendStateReflect>,
  /// [ColorWrites] bits
  pub write_mask: u32,
  pub front_face: FrontFaceReflect,
  pub cull_mode: Option<FaceReflect>,
}

impl From<&RasterizationStates> for RasterizationStatesReflect {
  fn from(v: &RasterizationStates) -> Self {
    Self {
      depth_write_enabled: v.depth_write_enabled,
      depth_compare: v.depth_compare,
      stencil: v.stencil.clone().into(),
      bias: v.bias.into(),
      blend: v.blend.map(Into::into),
      write_mask: v.write_mask.bits(),
      front_face: v.front_face.into(),
      cull_mode: v.cull_mode.map(Into::into),
    }
  }
}

impl From<RasterizationStatesReflect> for RasterizationStates {
  fn from(v: RasterizationStatesReflect) -> Self {
    Self {
      depth_write_enabled: v.depth_write_enabled,
      depth_compare: v.depth_compare,
      stencil: v.stencil.into(),
      bias: v.bias.into(),
      blend: v.blend.map(Into::into),
      write_mask: ColorWrites::from_bits_retain(v.write_mask),
      front_face: v.front_face.into(),
      cull_mode: v.cull_mode.map(Into::into),
    }
  }
}

macro_rules! mirror_struct {
  ($mirror: ident, $target: ty, { $($field: ident: $ty: ty),* $(,)? }) => {
    #[derive(Facet, Debug, Clone, Copy, PartialEq, Default)]
    pub struct $mirror {
      $(pub $field: $ty,)*
    }
    impl From<$target> for $mirror {
      fn from(v: $target) -> Self {
        Self { $($field: v.$field.into(),)* }
      }
    }
    impl From<$mirror> for $target {
      fn from(v: $mirror) -> Self {
        Self { $($field: v.$field.into(),)* }
      }
    }
  };
}

macro_rules! mirror_enum {
  ($mirror: ident, $target: ty, $default: ident, { $($variant: ident = $value: expr),* $(,)? }) => {
    #[repr(u32)]
    #[derive(Facet, Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum $mirror {
      #[default]
      $default = <$target>::$default as u32,
      $($variant = $value,)*
    }
    impl From<$target> for $mirror {
      fn from(v: $target) -> Self {
        match v {
          <$target>::$default => Self::$default,
          $(<$target>::$variant => Self::$variant,)*
        }
      }
    }
    impl From<$mirror> for $target {
      fn from(v: $mirror) -> Self {
        match v {
          $mirror::$default => Self::$default,
          $($mirror::$variant => Self::$variant,)*
        }
      }
    }
  };
}

mirror_struct!(StencilStateReflect, StencilState, {
  front: StencilFaceStateReflect,
  back: StencilFaceStateReflect,
  read_mask: u32,
  write_mask: u32,
});

mirror_struct!(StencilFaceStateReflect, StencilFaceState, {
  compare: CompareFunctionReflect,
  fail_op: StencilOperationReflect,
  depth_fail_op: StencilOperationReflect,
  pass_op: StencilOperationReflect,
});

mirror_struct!(DepthBiasStateReflect, DepthBiasState, {
  constant: i32,
  slope_scale: f32,
  clamp: f32,
});

mirror_struct!(BlendStateReflect, BlendState, {
  color: BlendComponentReflect,
  alpha: BlendComponentReflect,
});

mirror_struct!(BlendComponentReflect, BlendComponent, {
  src_factor: BlendFactorReflect,
  dst_factor: BlendFactorReflect,
  operation: BlendOperationReflect,
});

mirror_enum!(CompareFunctionReflect, CompareFunction, Always, {
  Never = 1,
  Less = 2,
  Equal = 3,
  LessEqual = 4,
  Greater = 5,
  NotEqual = 6,
  GreaterEqual = 7,
});

mirror_enum!(StencilOperationReflect, StencilOperation, Keep, {
  Zero = 1,
  Replace = 2,
  Invert = 3,
  IncrementClamp = 4,
  DecrementClamp = 5,
  IncrementWrap = 6,
  DecrementWrap = 7,
});

mirror_enum!(BlendFactorReflect, BlendFactor, One, {
  Zero = 0,
  Src = 2,
  OneMinusSrc = 3,
  SrcAlpha = 4,
  OneMinusSrcAlpha = 5,
  Dst = 6,
  OneMinusDst = 7,
  DstAlpha = 8,
  OneMinusDstAlpha = 9,
  SrcAlphaSaturated = 10,
  Constant = 11,
  OneMinusConstant = 12,
  Src1 = 13,
  OneMinusSrc1 = 14,
  Src1Alpha = 15,
  OneMinusSrc1Alpha = 16,
});

mirror_enum!(BlendOperationReflect, BlendOperation, Add, {
  Subtract = 1,
  ReverseSubtract = 2,
  Min = 3,
  Max = 4,
});

mirror_enum!(FrontFaceReflect, FrontFace, Ccw, {
  Cw = 1,
});

mirror_enum!(FaceReflect, Face, Back, {
  Front = 0,
});
