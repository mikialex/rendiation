use crate::{
  Arena, Handle, SceneGeometryData, SceneGraphBackend, SceneShadingData,
  SceneShadingParameterGroupData,
};

type ResourceArena<T> = Arena<ResourceWrap<T>>;

pub struct ResourceManager<T: SceneGraphBackend> {
  pub geometries: ResourceArena<SceneGeometryData<T>>,
  pub shadings: ResourceArena<SceneShadingData<T>>,
  pub shading_parameter_groups: ResourceArena<SceneShadingParameterGroupData<T>>,

  pub uniforms: ResourceArena<T::UniformBuffer>,
  pub uniform_values: ResourceArena<T::UniformValue>,

  pub textures: ResourceArena<T::VertexBuffer>,

  pub index_buffers: ResourceArena<T::IndexBuffer>,
  pub vertex_buffers: ResourceArena<T::VertexBuffer>,
}

/// wrap any resource with an index;
pub struct ResourceWrap<T> {
  index: Handle<Self>,
  resource: T,
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

impl<T: SceneGraphBackend> ResourceManager<T> {
  pub fn new() -> Self {
    Self {
      geometries: Arena::new(),
      shadings: Arena::new(),
      shading_parameter_groups: Arena::new(),
      uniforms: Arena::new(),
      uniform_values: Arena::new(),
      textures: Arena::new(),
      index_buffers: Arena::new(),
      vertex_buffers: Arena::new(),
    }
  }
}
