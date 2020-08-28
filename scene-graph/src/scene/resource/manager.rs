use crate::{
  Arena, BindGroupManager, Handle, RALBackend, SceneGeometryData, SceneShadingData,
  SceneShadingParameterGroupData, UBOManager,
};
use std::any::Any;

type ResourceArena<T> = Arena<ResourceWrap<T>>;

pub struct ResourceManager<T: RALBackend> {
  pub geometries: ResourceArena<SceneGeometryData<T>>,
  pub shadings: ResourceArena<SceneShadingData<T>>,

  pub bindgroups: BindGroupManager<T>,
  pub shading_parameter_groups: ResourceArena<SceneShadingParameterGroupData<T>>,

  pub bindable: Box<ShaderBindableResourceManager<T>>,

  pub index_buffers: ResourceArena<T::IndexBuffer>,
  pub vertex_buffers: ResourceArena<T::VertexBuffer>,
}

pub struct ShaderBindableResourceManager<T: RALBackend> {
  pub uniform_buffers: UBOManager<T>,
  pub uniform_values: ResourceArena<T::UniformValue>,
  pub samplers: ResourceArena<T::Sampler>,
  pub textures: ResourceArena<T::Texture>,
}

impl<T: RALBackend> ShaderBindableResourceManager<T> {
  pub fn new() -> Self {
    Self {
      uniform_buffers: UBOManager::new(),
      uniform_values: Arena::new(),
      textures: Arena::new(),
      samplers: Arena::new(),
    }
  }

  pub fn as_resource(boxed: &Box<Self>) -> &Box<dyn Any> {
    boxed
  }
}

/// wrap any resource with an index;
pub struct ResourceWrap<T> {
  index: Handle<Self>,
  resource: T,
}

impl<T: RALBackend> ResourceManager<T> {
  pub fn new() -> Self {
    Self {
      geometries: Arena::new(),
      shadings: Arena::new(),
      bindgroups: BindGroupManager::new(),
      bindable: Box::new(ShaderBindableResourceManager::new()),
      shading_parameter_groups: Arena::new(),
      index_buffers: Arena::new(),
      vertex_buffers: Arena::new(),
    }
  }

  pub fn maintain_gpu(&mut self, renderer: &mut T::Renderer) {
    self.bindable.uniform_buffers.maintain_gpu(renderer);
    self.bindgroups.maintain_gpu(renderer, &self.bindable)
  }
}

impl<T> ResourceWrap<T> {
  pub fn index(&self) -> Handle<Self> {
    self.index
  }

  pub fn resource(&self) -> &T {
    &self.resource
  }

  pub fn resource_mut(&mut self) -> &mut T {
    &mut self.resource
  }

  pub fn new_wrap(arena: &mut Arena<Self>, resource: T) -> &mut Self {
    let wrapped = Self {
      index: Handle::from_raw_parts(0, 0),
      resource,
    };
    let index = arena.insert(wrapped);
    let w = arena.get_mut(index).unwrap();
    w.index = index;
    w
  }
}
