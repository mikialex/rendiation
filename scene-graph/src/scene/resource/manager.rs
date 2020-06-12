use crate::{
  Arena, Index, SceneGraphBackEnd, SceneShading, SceneShadingParameterGroup, SceneGeometryData,
};

pub struct ResourceManager<T: SceneGraphBackEnd> {
  // pub resources: Vec<Box<dyn ResourceArena>>,

  pub geometries: Arena<ResouceWrap<SceneGeometryData<T>>>,
  pub shadings: Arena<SceneShading<T>>,
  pub shading_parameter_groups: Arena<SceneShadingParameterGroup<T>>,
  pub uniforms: Arena<ResouceWrap<T::UniformBuffer>>,
  pub index_buffers: Arena<ResouceWrap<T::IndexBuffer>>,
  pub vertex_buffers: Arena<ResouceWrap<T::VertexBuffer>>,
  // pub textures: Arena<ResouceWrap<T::Texture>>,
}

// try reduce boilplate code, wip
// pub trait Resource{
//   fn type_index(&self) -> usize;
// }

// impl<T: SceneGraphBackEnd> Resource for SceneGeometryData<T>{
//   fn type_index(&self) -> usize{
//     0
//   }
// }

// pub trait ResourceArena{

// }

// impl<T: SceneGraphBackEnd> ResourceManager<T>{
//   pub fn get_resource_wrap<U:Resource>(&mut self, resource: U){
//     let arena = self.resources[resource.type_index()];

//   }
// }

/// wrap any resouce with an index;
pub struct ResouceWrap<T> {
  index: Index,
  resource: T,
}

impl<T> ResouceWrap<T> {
  pub fn index(&self) -> Index {
    self.index
  }

  pub fn resource(&self) -> &T {
    &self.resource
  }

  pub fn resource_mut(&mut self) -> &mut T {
    &mut self.resource
  }

  pub fn new_wrap(arena: &mut Arena<Self>, resource: T) -> &mut Self{
    let wrapped = Self {
      index: Index::from_raw_parts(0, 0),
      resource,
    };
    let index = arena.insert(wrapped);
    let w = arena.get_mut(index).unwrap();
    w.index = index;
    w
  }
}

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn new() -> Self {
    Self {
      geometries: Arena::new(),
      shadings: Arena::new(),
      shading_parameter_groups: Arena::new(),
      uniforms: Arena::new(),
      // textures: Arena::new(),
      index_buffers: Arena::new(),
      vertex_buffers: Arena::new(),
    }
  }
}
