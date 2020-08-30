use crate::{GeometryHandle, RALBackend, Scene, ShadingHandle};
use arena::Handle;
use rendiation_ral::{AnyPlaceHolder, ShadingProvider};
// use rendiation_render_entity::BoundingData;

pub type RenderObjectHandle<T> = Handle<RenderObject<T>>;

pub struct RenderObject<T: RALBackend> {
  pub shading_index: ShadingHandle<T, AnyPlaceHolder>,
  pub geometry_index: GeometryHandle<T>,
  pub render_order: i32, // todo for sorting
}

// impl<T: RALBackend> RenderObject<T> {
//   pub fn get_bounding_local<'a>(&self, scene: &'a Scene<T>) -> &'a Option<BoundingData> {
//     &scene
//       .resources
//       .get_geometry(self.geometry_index)
//       .resource()
//       .bounding_local
//   }
// }

impl<T: RALBackend> Scene<T> {
  pub fn create_render_object<S: ShadingProvider<T>>(
    &mut self,
    geometry_index: GeometryHandle<T>,
    shading_index: ShadingHandle<T, S>,
  ) -> RenderObjectHandle<T> {
    let obj = unsafe {
      RenderObject {
        render_order: 0,
        shading_index: shading_index.cast_type(),
        geometry_index,
      }
    };
    self.render_objects.insert(obj)
  }

  pub fn delete_render_object(&mut self, index: RenderObjectHandle<T>) {
    self.render_objects.remove(index);
  }
}
