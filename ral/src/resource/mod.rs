use crate::RALBackend;
use arena::Handle;
use std::{marker::PhantomData, ops::Range};

pub mod bindgroup;
pub mod geometry;
pub mod manager;
pub mod ref_storage;
pub mod shading;
pub mod uniform;

pub use bindgroup::*;
pub use geometry::*;
pub use manager::*;
pub use shading::*;
pub use uniform::*;

pub struct RenderObject<
  T: RALBackend,
  G: GeometryProvider<T> = AnyGeometryProvider,
  SP: ShadingProvider<T, Geometry = G> = AnyPlaceHolder,
> {
  pub shading: ShadingHandle<T, SP>,
  pub geometry: GeometryHandle<T, G>,
}

impl<T: RALBackend> RenderObject<T> {
  pub fn new<SP: ShadingProvider<T>, G: GeometryProvider<T>>(
    geometry: GeometryHandle<T, G>,
    shading: ShadingHandle<T, SP>,
  ) -> Self {
    Self {
      shading: unsafe { shading.cast_type() },
      geometry: unsafe { geometry.cast_type() },
    }
  }
}

pub type ShadingHandle<R, T> = Handle<ShadingPair<R, T>>;
pub type BindGroupHandle<R, T> = Handle<BindgroupPair<R, T>>;

pub type SamplerHandle<T> = Handle<<T as RALBackend>::Sampler>;
pub type TextureHandle<T> = Handle<<T as RALBackend>::Texture>;

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
pub type GeometryHandle<T, G> = Handle<GeometryResourceInstance<T, G>>;

pub struct AnyPlaceHolder;

pub trait RALBindgroupHandle<T: RALBackend> {
  type HandleType;
}
impl<T: RALBackend, U: UBOData> RALBindgroupHandle<T> for U {
  type HandleType = UniformHandle<T, U>;
}
impl<'a, T: RALBackend, U: UBOData> RALBindgroupItem<'a, T> for U {
  type Resource = UniformBufferRef<'a, T, U>;
  fn get_item(
    handle: Self::HandleType,
    resources: &'a ShaderBindableResourceManager<T>,
  ) -> Self::Resource {
    resources.uniform_buffers.get_uniform_gpu(handle)
  }
}

pub struct UniformBufferRef<'a, T: RALBackend, U: 'static + Sized> {
  pub ty: PhantomData<U>,
  pub gpu: (&'a T::UniformBuffer, Range<u64>),
  pub data: &'a U,
}

pub trait UBOData: 'static + Sized {}

pub trait RALBindgroupItem<'a, T: RALBackend>: RALBindgroupHandle<T> {
  type Resource;
  fn get_item(
    handle: Self::HandleType,
    resources: &'a ShaderBindableResourceManager<T>,
  ) -> Self::Resource;
}

pub trait BindGroupCreator<T: RALBackend>: BindGroupProvider<T> {
  fn create_bindgroup(
    instance: &Self::Instance,
    renderer: &T::Renderer,
    resources: &ShaderBindableResourceManager<T>,
  ) -> T::BindGroup;
}

pub trait BindGroupProvider<T: RALBackend>: 'static {
  type Instance;
  fn apply(
    instance: &Self::Instance,
    gpu_bindgroup: &T::BindGroup,
    index: usize,
    shading: &T::Shading,
    resources: &ShaderBindableResourceManager<T>,
    render_pass: &mut T::RenderPass,
  );
}

pub trait ShadingProvider<T: RALBackend>: 'static + Sized {
  type Instance;
  type Geometry: GeometryProvider<T>;
  fn apply(
    instance: &Self::Instance,
    gpu_shading: &T::Shading,
    render_pass: &mut T::RenderPass,
    resources: &ResourceManager<T>,
  );
}

pub trait GeometryProvider<T: RALBackend>: 'static + Sized {
  // type Instance;
  // fn apply(
  //   instance: &Self::Instance,
  //   render_pass: &mut T::RenderPass,
  //   resources: &ResourceManager<T>,
  // );
  // fn get_primitive_topology();
}

// pub trait GeometryVertexProvider<T: RALBackend> {
//   type Instance;
//   fn apply(
//     instance: &Self::Instance,
//     render_pass: &mut T::RenderPass,
//     resources: &ResourceManager<T>,
//   );
// }
