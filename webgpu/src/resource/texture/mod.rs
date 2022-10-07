pub mod d2;
pub use d2::*;
pub mod cube;
pub use cube::*;

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

impl BindableResourceView for gpu::TextureView {
  fn as_bindable(&self) -> gpu::BindingResource {
    todo!()
  }
}

#[derive(Clone)]
pub struct GPU1DTexture(pub GPUTexture);

#[derive(Clone)]
pub struct GPU2DTexture(pub GPUTexture);

#[derive(Clone)]
pub struct GPU3DTexture(pub GPUTexture);

#[derive(Clone)]
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

#[derive(Clone)]
pub struct GPU1DTextureView(pub GPUTextureView);
#[derive(Clone)]
pub struct GPU1DArrayTextureView(pub GPUTextureView);
#[derive(Clone)]
pub struct GPU2DTextureView(pub GPUTextureView);
#[derive(Clone)]
pub struct GPU2DArrayTextureView(pub GPUTextureView);
#[derive(Clone)]
pub struct GPUCubeTextureView(pub GPUTextureView);
#[derive(Clone)]
pub struct GPUCubeArrayTextureView(pub GPUTextureView);
#[derive(Clone)]
pub struct GPU3DTextureView(pub GPUTextureView);

macro_rules! texture_view_inner {
  ($ty: ty) => {
    impl BindingSource for $ty {
      type Uniform = GPUTextureView;

      fn get_uniform(&self) -> Self::Uniform {
        self.0.get_uniform()
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
texture_view_inner!(GPU1DArrayTextureView);
texture_view_inner!(GPU2DTextureView);
texture_view_inner!(GPU2DArrayTextureView);
texture_view_inner!(GPUCubeTextureView);
texture_view_inner!(GPUCubeArrayTextureView);
texture_view_inner!(GPU3DTextureView);

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

// todo check their view layer count
texture_view_downcast!(
  GPU1DTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D1,
  "raw texture view not a 1d"
);
texture_view_downcast!(
  GPU1DArrayTextureView,
  value,
  value.resource.desc.dimension == gpu::TextureDimension::D1,
  "raw texture view not a 1d array"
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
