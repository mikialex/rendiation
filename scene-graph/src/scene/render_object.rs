use rendiation::*;
use generational_arena::Index;
use crate::Scene;
use rendiation_render_entity::BoundingData;

pub struct RenderObject {
  pub shading_index: Index,
  pub geometry_index: Index,
  pub render_order: i32, // todo for sorting
}

impl RenderObject {
  pub fn render<'a, 'b: 'a>(&self, pass: &mut WGPURenderPass<'a>, scene: &'b Scene) {
    let shading = scene.resources.get_shading(self.shading_index);
    let geometry = scene.resources.get_geometry(self.geometry_index);

    pass.set_pipeline(shading.get_gpu_pipeline());
    pass.set_index_buffer(geometry.get_gpu_index_buffer());
    for i in 0..geometry.vertex_buffer_count() {
      let buffer = geometry.get_gpu_vertex_buffer(i);
      pass.set_vertex_buffer(i, buffer);
    }

    for i in 0..shading.get_bindgroup_count() {
      let bindgroup = scene.resources.get_bindgroup(shading.get_bindgroup(i));
      pass.set_bindgroup(i, bindgroup);
    }

    pass.draw_indexed(geometry.get_draw_range())
  }

  pub fn get_bounding_local(&self, scene: &Scene) -> &BoundingData{
    todo!()
  }
}
