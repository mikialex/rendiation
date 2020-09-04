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

pub struct AnyPlaceHolder;

pub type ShadingHandle<R, T> = Handle<ShadingPair<R, T>>;
pub type BindGroupHandle<R, T> = Handle<BindgroupPair<R, T>>;

pub type SamplerHandle<T> = Handle<<T as RALBackend>::Sampler>;
pub type TextureHandle<T> = Handle<<T as RALBackend>::Texture>;
pub type TextureViewHandle<T> = Handle<<T as RALBackend>::TextureView>;

pub struct UniformHandle<R: RALBackend, U> {
  index: usize,
  phantom: PhantomData<U>,
  phantom2: PhantomData<R>,
}

impl<R: RALBackend, T> Clone for UniformHandle<R, T> {
  fn clone(&self) -> Self {
    *self
  }
}
impl<R: RALBackend, T> Copy for UniformHandle<R, T> {}

pub type IndexBufferHandle<T> = Handle<ResourceWrap<<T as RALBackend>::IndexBuffer>>;
pub type VertexBufferHandle<T> = Handle<ResourceWrap<<T as RALBackend>::VertexBuffer>>;
pub type GeometryHandle<T> = Handle<ResourceWrap<SceneGeometryData<T>>>;
