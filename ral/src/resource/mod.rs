use crate::RALBackend;
use arena::Handle;
use std::marker::PhantomData;

pub mod bindgroup;
pub mod geometry;
pub mod manager;
pub mod shading;
pub mod uniform;

pub use bindgroup::*;
pub use geometry::*;
pub use manager::*;
pub use shading::*;
pub use uniform::*;

pub type ShadingHandle<R, T> = Handle<ShadingPair<R, T>>;
pub type BindGroupHandle<R, T> = Handle<BindgroupPair<R, T>>;
pub type SamplerHandle<T> = Handle<ResourceWrap<<T as RALBackend>::Sampler>>;
pub type TextureHandle<T> = Handle<ResourceWrap<<T as RALBackend>::Texture>>;
pub type SampledTextureHandle<T> = Handle<ResourceWrap<<T as RALBackend>::Texture>>;
pub struct UniformHandle<U> {
  index: usize,
  phantom: PhantomData<U>,
}

impl<T> Clone for UniformHandle<T> {
  fn clone(&self) -> Self {
    *self
  }
}
impl<T> Copy for UniformHandle<T> {}

pub type IndexBufferHandle<T> = Handle<ResourceWrap<<T as RALBackend>::IndexBuffer>>;
pub type VertexBufferHandle<T> = Handle<ResourceWrap<<T as RALBackend>::VertexBuffer>>;
pub type GeometryHandle<T> = Handle<ResourceWrap<SceneGeometryData<T>>>;
