use crate::{GeometryHandle, RALBackend, Scene, ShadingHandle};
use arena::Handle;
use rendiation_render_entity::BoundingData;

pub type RenderObjectHandle<T> = Handle<RenderObject<T>>;

pub struct RenderObject<T: RALBackend> {
  pub shading_index: ShadingHandle<T>,
  pub geometry_index: GeometryHandle<T>,
  pub render_order: i32, // todo for sorting
}

impl<T: RALBackend> RenderObject<T> {
  pub fn get_bounding_local<'a>(&self, scene: &'a Scene<T>) -> &'a Option<BoundingData> {
    &scene
      .resources
      .get_geometry(self.geometry_index)
      .resource()
      .bounding_local
  }
}

impl<T: RALBackend> Scene<T> {
  pub fn create_render_object(
    &mut self,
    geometry_index: GeometryHandle<T>,
    shading_index: ShadingHandle<T>,
  ) -> RenderObjectHandle<T> {
    let obj = RenderObject {
      render_order: 0,
      shading_index,
      geometry_index,
    };
    self.render_objects.insert(obj)
  }

  pub fn delete_render_object(&mut self, index: RenderObjectHandle<T>) {
    self.render_objects.remove(index);
  }
}
