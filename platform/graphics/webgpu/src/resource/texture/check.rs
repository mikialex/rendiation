// todo, we should ask wgpu to expose more correct validation methods

use crate::*;

pub trait DimensionDynamicViewCheck {
  fn check(view_desc: &gpu::TextureViewDescriptor, desc: &gpu::TextureDescriptor) -> bool;
}

/// the same logic as wgpu resolves the view dimension when the view desc not specify it.
///
/// the array layer count is not checked here, because the wgpu will validate the layer count
/// against the resolved view dimension when creating the view.
fn resolve_view_dimension(
  view_desc: &gpu::TextureViewDescriptor,
  desc: &gpu::TextureDescriptor,
) -> gpu::TextureViewDimension {
  view_desc.dimension.unwrap_or(match desc.dimension {
    gpu::TextureDimension::D1 => gpu::TextureViewDimension::D1,
    gpu::TextureDimension::D2 => {
      if desc.array_layer_count() == 1 {
        gpu::TextureViewDimension::D2
      } else {
        gpu::TextureViewDimension::D2Array
      }
    }
    gpu::TextureDimension::D3 => gpu::TextureViewDimension::D3,
  })
}

macro_rules! impl_dimension_check {
  ($ty: ty, $view_dimension: expr) => {
    impl DimensionDynamicViewCheck for $ty {
      fn check(view_desc: &gpu::TextureViewDescriptor, desc: &gpu::TextureDescriptor) -> bool {
        resolve_view_dimension(view_desc, desc) == $view_dimension
      }
    }
  };
}

impl_dimension_check!(TextureDimension1, gpu::TextureViewDimension::D1);
impl_dimension_check!(TextureDimension2, gpu::TextureViewDimension::D2);
impl_dimension_check!(TextureDimension3, gpu::TextureViewDimension::D3);
impl_dimension_check!(TextureDimension2Array, gpu::TextureViewDimension::D2Array);
impl_dimension_check!(TextureDimensionCube, gpu::TextureViewDimension::Cube);
impl_dimension_check!(
  TextureDimensionCubeArray,
  gpu::TextureViewDimension::CubeArray
);

pub trait TextureFormatDynamicCheck {
  // todo, we should record the device features info in desc
  fn check(format: &gpu::TextureFormat, aspect: TextureAspect, sample_count: u32) -> bool {
    if sample_count != 1 {
      return false;
    }
    if let Some(ty) = format.sample_type(Some(aspect), None) {
      Self::check_impl(ty)
    } else {
      false
    }
  }

  fn check_impl(ty: TextureSampleType) -> bool;
}

impl<T: TextureFormatDynamicCheck> TextureFormatDynamicCheck for MultiSampleOf<T> {
  fn check(format: &gpu::TextureFormat, aspect: TextureAspect, sample_count: u32) -> bool {
    if sample_count <= 1 {
      return false;
    }
    T::check(format, aspect, 1)
  }

  fn check_impl(_: TextureSampleType) -> bool {
    unreachable!()
  }
}

impl TextureFormatDynamicCheck for f32 {
  fn check_impl(ty: TextureSampleType) -> bool {
    matches!(ty, gpu::TextureSampleType::Float { .. }) | matches!(ty, gpu::TextureSampleType::Depth)
  }
}
impl TextureFormatDynamicCheck for u32 {
  fn check_impl(ty: TextureSampleType) -> bool {
    matches!(ty, gpu::TextureSampleType::Uint)
  }
}
impl TextureFormatDynamicCheck for i32 {
  fn check_impl(ty: TextureSampleType) -> bool {
    matches!(ty, gpu::TextureSampleType::Sint)
  }
}
impl TextureFormatDynamicCheck for TextureSampleDepth {
  fn check_impl(ty: TextureSampleType) -> bool {
    matches!(ty, gpu::TextureSampleType::Depth)
  }
}
