mod d2;
pub use d2::*;
mod cube;
mod storage;
pub use storage::*;

use crate::*;

pub type GPUTexture = ResourceRc<gpu::Texture>;
pub type GPUTextureView = ResourceViewRc<gpu::Texture>;

impl Resource for gpu::Texture {
  type Descriptor = gpu::TextureDescriptor<'static>;

  type View = gpu::TextureView;

  type ViewDescriptor = gpu::TextureViewDescriptor<'static>;

  fn create_view(&self, desc: &Self::ViewDescriptor) -> Self::View {
    self.create_view(desc)
  }
}

impl InitResourceByAllocation for gpu::Texture {
  fn create_resource(desc: &Self::Descriptor, device: &GPUDevice) -> Self {
    device.create_texture(desc)
  }
}

impl BindableResourceProvider for GPUTextureView {
  fn get_bindable(&self) -> BindingResourceOwned {
    BindingResourceOwned::TextureView(self.clone())
  }
}
impl BindableResourceView for gpu::TextureView {
  fn as_bindable(&self) -> gpu::BindingResource {
    gpu::BindingResource::TextureView(self)
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GPU1DTexture(pub GPUTexture);

#[derive(Clone, Debug, PartialEq)]
pub struct GPU2DTexture(pub GPUTexture);

#[derive(Clone, Debug, PartialEq)]
pub struct GPU3DTexture(pub GPUTexture);

#[derive(Clone, Debug, PartialEq)]
pub struct GPUCubeTexture(pub GPUTexture);

macro_rules! texture_inner {
  ($ty: ty) => {
    impl Deref for $ty {
      type Target = GPUTexture;

      fn deref(&self) -> &Self::Target {
        &self.0
      }
    }
  };
}

texture_inner!(GPU1DTexture);
texture_inner!(GPU2DTexture);
texture_inner!(GPU3DTexture);
texture_inner!(GPUCubeTexture);

macro_rules! texture_downcast {
  ($ty: ty, $var:tt, $check: expr, $err: tt) => {
    impl TryFrom<GPUTexture> for $ty {
      type Error = &'static str;

      fn try_from($var: GPUTexture) -> Result<Self, Self::Error> {
        if $check {
          Ok(Self($var))
        } else {
          Err("raw texture not a 1d")
        }
      }
    }
  };
}

texture_downcast!(
  GPU1DTexture,
  value,
  value.desc.dimension == gpu::TextureDimension::D1,
  "raw texture not a 1d"
);
texture_downcast!(
  GPU2DTexture,
  value,
  value.desc.dimension == gpu::TextureDimension::D2,
  "raw texture not a 2d"
);
texture_downcast!(
  GPU3DTexture,
  value,
  value.desc.dimension == gpu::TextureDimension::D3,
  "raw texture not a 3d"
);
texture_downcast!(
  GPUCubeTexture,
  value,
  value.desc.dimension == gpu::TextureDimension::D2 && value.desc.array_layer_count() == 6,
  "raw texture not a cube"
);

#[derive(Clone, Debug, PartialEq)]
pub struct GPU1DTextureView(pub GPUTextureView);
#[derive(Clone, Debug, PartialEq)]
pub struct GPU2DTextureView(pub GPUTextureView);

impl GPU2DTextureView {
  pub fn size(&self) -> Size {
    let size = self
      .resource
      .desc
      .size
      .mip_level_size(self.desc.base_mip_level, gpu::TextureDimension::D2);
    GPUTextureSize::from_gpu_size(size)
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GPU2DArrayTextureView(pub GPUTextureView);
#[derive(Clone, Debug, PartialEq)]
pub struct GPUCubeTextureView(pub GPUTextureView);
#[derive(Clone, Debug, PartialEq)]
pub struct GPUCubeArrayTextureView(pub GPUTextureView);
#[derive(Clone, Debug, PartialEq)]
pub struct GPU3DTextureView(pub GPUTextureView);

#[derive(Clone, Debug, PartialEq)]
pub struct GPU2DDepthTextureView(pub GPUTextureView);
#[derive(Clone, Debug, PartialEq)]
pub struct GPU2DArrayDepthTextureView(pub GPUTextureView);
#[derive(Clone, Debug, PartialEq)]
pub struct GPUCubeDepthTextureView(pub GPUTextureView);
#[derive(Clone, Debug, PartialEq)]
pub struct GPUCubeArrayDepthTextureView(pub GPUTextureView);

#[derive(Clone, Debug, PartialEq)]
pub struct GPUMultiSample2DTextureView(pub GPUTextureView);
#[derive(Clone, Debug, PartialEq)]
pub struct GPUMultiSample2DDepthTextureView(pub GPUTextureView);

macro_rules! texture_view_inner {
  ($ty: ty) => {
    impl CacheAbleBindingSource for $ty {
      fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
        self.0.get_binding_build_source()
      }
    }

    impl Deref for $ty {
      type Target = GPUTextureView;

      fn deref(&self) -> &Self::Target {
        &self.0
      }
    }
  };
}

texture_view_inner!(GPU1DTextureView);
texture_view_inner!(GPU2DTextureView);
texture_view_inner!(GPU2DArrayTextureView);
texture_view_inner!(GPUCubeTextureView);
texture_view_inner!(GPUCubeArrayTextureView);
texture_view_inner!(GPU3DTextureView);

texture_view_inner!(GPU2DDepthTextureView);
texture_view_inner!(GPU2DArrayDepthTextureView);
texture_view_inner!(GPUCubeDepthTextureView);
texture_view_inner!(GPUCubeArrayDepthTextureView);

texture_view_inner!(GPUMultiSample2DTextureView);
texture_view_inner!(GPUMultiSample2DDepthTextureView);

macro_rules! texture_view_downcast {
  ($ty: ty, $var:tt, $check: expr, $err: tt) => {
    impl TryFrom<GPUTextureView> for $ty {
      type Error = &'static str;

      fn try_from($var: GPUTextureView) -> Result<Self, Self::Error> {
        if $check {
          Ok(Self($var))
        } else {
          Err("raw texture not a 1d")
        }
      }
    }
  };
}

// todo check view desc dimension
texture_view_downcast!(
  GPU1DTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D1,
  "raw texture view not a 1d"
);
texture_view_downcast!(
  GPU2DTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D2,
  "raw texture view not a 2d"
);
texture_view_downcast!(
  GPU2DArrayTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D2,
  "raw texture view not a 2d array"
);
texture_view_downcast!(
  GPU3DTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D3,
  "raw texture view not a 3d"
);
texture_view_downcast!(
  GPUCubeTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D2
    && value.resource.desc.array_layer_count() == 6,
  "raw texture view not a cube"
);
texture_view_downcast!(
  GPUCubeArrayTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D2
    && value.resource.desc.array_layer_count() == 6,
  "raw texture view not a cube array"
);

// todo check depth format
// todo check view desc dimension
texture_view_downcast!(
  GPU2DDepthTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D2,
  "raw texture view not a 2d depth"
);
texture_view_downcast!(
  GPUMultiSample2DDepthTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D2
    && value.resource.desc.sample_count > 1,
  "raw texture view not a 2d depth"
);
texture_view_downcast!(
  GPU2DArrayDepthTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D2,
  "raw texture view not a 2d array depth"
);
texture_view_downcast!(
  GPUCubeDepthTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D2
    && value.resource.desc.array_layer_count() == 6,
  "raw texture view not a cube"
);
texture_view_downcast!(
  GPUCubeArrayDepthTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D2
    && value.resource.desc.array_layer_count() == 6,
  "raw texture view not a cube array"
);
