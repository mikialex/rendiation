use crate::{ShaderSampler, ShaderTexture, RAL};
use arena::Handle;
use std::{marker::PhantomData, ops::Range};

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

pub struct Drawcall<T>
where
  T: RAL,
{
  shading: ShadingHandle<T, AnyPlaceHolder>,
  geometry: GeometryHandle<T, AnyGeometryProvider>,
}

impl<T: RAL> ResourceManager<T> {
  pub fn get_resource(
    &self,
    drawcall: &Drawcall<T>,
  ) -> (&dyn ShadingStorageTrait<T>, &dyn GeometryResource<T>) {
    (
      self.shadings.get_shading_boxed(drawcall.shading),
      self.get_geometry_boxed(drawcall.geometry),
    )
  }
}

impl<T: RAL> Drawcall<T> {
  pub fn new<SP, G>(geometry: GeometryHandle<T, G>, shading: ShadingHandle<T, SP>) -> Self
  where
    SP: ShadingProvider<T>,
    G: GeometryProvider,
  {
    Self {
      shading: unsafe { shading.cast_type() },
      geometry: unsafe { geometry.cast_type() },
    }
  }
}

pub type ShadingHandle<R, T> = Handle<ShadingPair<R, T>>;
pub type BindGroupHandle<R, T> = Handle<BindgroupPair<R, T>>;
pub struct AnyBindGroupType;
impl<T: RAL> BindGroupProvider<T> for AnyBindGroupType {
  type Instance = ();

  fn apply(
    _instance: &Self::Instance,
    _gpu_bindgroup: &T::BindGroup,
    _index: usize,
    _shading: &T::Shading,
    _resources: &ShaderBindableResourceManager<T>,
    _render_pass: &mut T::RenderPass,
  ) {
    unreachable!()
  }
  fn add_reference(
    _instance: &Self::Instance,
    _bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>,
    _resources: &mut ShaderBindableResourceManager<T>,
  ) {
  }
  fn remove_reference(
    _instance: &Self::Instance,
    _bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>,
    _resources: &mut ShaderBindableResourceManager<T>,
  ) {
  }
}

pub type SamplerHandle<T> = Handle<<T as RAL>::Sampler>;
pub type TextureHandle<T> = Handle<<T as RAL>::Texture>;

pub struct UniformHandle<R: RAL, U> {
  index: usize,
  phantom: PhantomData<U>,
  phantom2: PhantomData<R>,
}

impl<R: RAL, T> Clone for UniformHandle<R, T> {
  fn clone(&self) -> Self {
    *self
  }
}
impl<R: RAL, T> Copy for UniformHandle<R, T> {}

pub type IndexBufferHandle<T> = Handle<ResourceWrap<<T as RAL>::IndexBuffer>>;
pub type VertexBufferHandle<T> = Handle<ResourceWrap<<T as RAL>::VertexBuffer>>;
pub type GeometryHandle<T, G> = Handle<GeometryResourceInstance<T, G>>;

pub struct AnyPlaceHolder;

pub trait RALBindgroupHandle<T: RAL> {
  type HandleType;
}
impl<T: RAL, U: UBOData> RALBindgroupHandle<T> for U {
  type HandleType = UniformHandle<T, U>;
}
impl<'a, T: RAL, U: UBOData> RALBindgroupItem<'a, T> for U {
  type Resource = UniformBufferRef<'a, T, U>;
  fn get_item(
    handle: Self::HandleType,
    resources: &'a ShaderBindableResourceManager<T>,
  ) -> Self::Resource {
    resources.uniform_buffers.get_uniform_gpu(handle)
  }
  fn add_reference(
    self_handle: Self::HandleType,
    bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>,
    resources: &'a mut ShaderBindableResourceManager<T>,
  ) {
    resources
      .uniform_buffers
      .get_storage_or_create::<U>()
      .add_reference(bindgroup_handle, self_handle.index)
  }
  fn remove_reference(
    self_handle: Self::HandleType,
    bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>,
    resources: &'a mut ShaderBindableResourceManager<T>,
  ) {
    resources
      .uniform_buffers
      .get_storage_or_create::<U>()
      .remove_reference(bindgroup_handle, self_handle.index)
  }
}

impl<T: RAL> RALBindgroupHandle<T> for ShaderTexture {
  type HandleType = TextureHandle<T>;
}
impl<'a, T: RAL> RALBindgroupItem<'a, T> for ShaderTexture {
  type Resource = &'a <T as RAL>::Texture;
  fn get_item(
    handle: Self::HandleType,
    resources: &'a ShaderBindableResourceManager<T>,
  ) -> Self::Resource {
    resources.textures.get(handle).unwrap()
  }
  fn add_reference(
    _self_handle: Self::HandleType,
    _bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>,
    _resources: &'a mut ShaderBindableResourceManager<T>,
  ) {
  }
  fn remove_reference(
    _self_handle: Self::HandleType,
    _bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>,
    _resources: &'a mut ShaderBindableResourceManager<T>,
  ) {
  }
}

impl<T: RAL> RALBindgroupHandle<T> for ShaderSampler {
  type HandleType = SamplerHandle<T>;
}
impl<'a, T: RAL> RALBindgroupItem<'a, T> for ShaderSampler {
  type Resource = &'a T::Sampler;
  fn get_item(
    handle: Self::HandleType,
    resources: &'a ShaderBindableResourceManager<T>,
  ) -> Self::Resource {
    resources.samplers.get(handle).unwrap()
  }
  fn add_reference(
    _self_handle: Self::HandleType,
    _bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>,
    _resources: &'a mut ShaderBindableResourceManager<T>,
  ) {
  }
  fn remove_reference(
    _self_handle: Self::HandleType,
    _bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>,
    _resources: &'a mut ShaderBindableResourceManager<T>,
  ) {
  }
}

pub struct UniformBufferRef<'a, T: RAL, U: 'static + Sized> {
  pub ty: PhantomData<U>,
  pub gpu: (&'a T::UniformBuffer, Range<u64>),
  pub data: &'a U,
}

pub trait UBOData: 'static + Sized {}

pub trait RALBindgroupItem<'a, T: RAL>: RALBindgroupHandle<T> {
  type Resource;

  fn get_item(
    handle: Self::HandleType,
    resources: &'a ShaderBindableResourceManager<T>,
  ) -> Self::Resource;
  fn add_reference(
    self_handle: Self::HandleType,
    bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>,
    resources: &'a mut ShaderBindableResourceManager<T>,
  );
  fn remove_reference(
    self_handle: Self::HandleType,
    bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>,
    resources: &'a mut ShaderBindableResourceManager<T>,
  );
}

pub trait BindGroupCreator<T: RAL>: BindGroupProvider<T> {
  fn create_bindgroup(
    instance: &Self::Instance,
    renderer: &T::Renderer,
    resources: &ShaderBindableResourceManager<T>,
  ) -> T::BindGroup;
}

pub trait BindGroupProvider<T: RAL>: 'static {
  type Instance;
  fn apply(
    instance: &Self::Instance,
    gpu_bindgroup: &T::BindGroup,
    index: usize,
    shading: &T::Shading,
    resources: &ShaderBindableResourceManager<T>,
    render_pass: &mut T::RenderPass,
  );

  fn add_reference(
    instance: &Self::Instance,
    bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>,
    resources: &mut ShaderBindableResourceManager<T>,
  );
  fn remove_reference(
    instance: &Self::Instance,
    bindgroup_handle: BindGroupHandle<T, AnyBindGroupType>,
    resources: &mut ShaderBindableResourceManager<T>,
  );
}

pub trait ShaderGeometryInfo {
  type Geometry: GeometryProvider;
}

pub trait ShadingProvider<T: RAL>: 'static + Sized + ShaderGeometryInfo {
  type Instance;
  fn apply(
    instance: &Self::Instance,
    gpu_shading: &T::Shading,
    render_pass: &mut T::RenderPass,
    resources: &ResourceManager<T>,
  );
}

pub trait ShadingCreator<T: RAL>: ShadingProvider<T> {
  fn create_shader(instance: &Self::Instance, renderer: &mut T::Renderer) -> T::Shading;
}

// just marker type for vertex
// not related to real geometry container type;
pub trait GeometryProvider: 'static + Sized {}
