// todo, we should ask wgpu to expose more correct validation methods

use crate::*;

pub trait DimensionDynamicViewCheck {
  fn check(view_desc: &gpu::TextureViewDescriptor, desc: &gpu::TextureDescriptor) -> bool;
}
impl DimensionDynamicViewCheck for TextureDimension1 {
  fn check(view_desc: &gpu::TextureViewDescriptor, desc: &gpu::TextureDescriptor) -> bool {
    let mut valid = true;

    if let Some(layer_count) = view_desc.array_layer_count {
      valid |= layer_count == 1;
    }

    if let Some(dimension) = view_desc.dimension {
      valid |= dimension == gpu::TextureViewDimension::D1
    } else {
      valid |= desc.dimension == gpu::TextureDimension::D1
    }
    valid
  }
}
impl DimensionDynamicViewCheck for TextureDimension2 {
  fn check(view_desc: &gpu::TextureViewDescriptor, desc: &gpu::TextureDescriptor) -> bool {
    let mut valid = true;

    if let Some(layer_count) = view_desc.array_layer_count {
      valid |= layer_count == 1;
    }

    if let Some(dimension) = view_desc.dimension {
      valid |= dimension == gpu::TextureViewDimension::D2
    } else {
      valid |= desc.dimension == gpu::TextureDimension::D2
    }
    valid
  }
}
impl DimensionDynamicViewCheck for TextureDimension3 {
  fn check(view_desc: &gpu::TextureViewDescriptor, desc: &gpu::TextureDescriptor) -> bool {
    let mut valid = true;

    if let Some(dimension) = view_desc.dimension {
      valid |= dimension == gpu::TextureViewDimension::D3
    } else {
      valid |= desc.dimension == gpu::TextureDimension::D3
    }
    valid
  }
}
impl DimensionDynamicViewCheck for TextureDimension2Array {
  fn check(view_desc: &gpu::TextureViewDescriptor, desc: &gpu::TextureDescriptor) -> bool {
    let mut valid = true;

    if let Some(dimension) = view_desc.dimension {
      valid |= dimension == gpu::TextureViewDimension::D2Array
    } else {
      valid |= desc.dimension == gpu::TextureDimension::D2
    }
    valid
  }
}
impl DimensionDynamicViewCheck for TextureDimensionCube {
  fn check(view_desc: &gpu::TextureViewDescriptor, desc: &gpu::TextureDescriptor) -> bool {
    let mut valid = true;

    if let Some(layer_count) = view_desc.array_layer_count {
      valid |= layer_count == 6;
    }

    if let Some(dimension) = view_desc.dimension {
      valid |= dimension == gpu::TextureViewDimension::Cube
    } else {
      valid |= desc.dimension == gpu::TextureDimension::D2
    }
    valid
  }
}
impl DimensionDynamicViewCheck for TextureDimensionCubeArray {
  fn check(view_desc: &gpu::TextureViewDescriptor, desc: &gpu::TextureDescriptor) -> bool {
    let mut valid = true;

    if let Some(layer_count) = view_desc.array_layer_count {
      valid |= layer_count >= 6 && layer_count % 6 == 0;
    }

    if let Some(dimension) = view_desc.dimension {
      valid |= dimension == gpu::TextureViewDimension::Cube
    } else {
      valid |= desc.dimension == gpu::TextureDimension::D2
    }
    valid
  }
}

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
