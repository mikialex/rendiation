use super::scene::Scene;
use generational_arena::Index;

pub struct RenderList {
  pub render_objects: Vec<Index>,
}

impl RenderList {
  pub fn new() -> Self {
    Self {
      render_objects: Vec::new(),
    }
  }

  pub fn clear(&mut self) -> &mut Self{
    self.render_objects.clear();
    self
  }

  pub fn push(&mut self, node_index: Index) -> &mut Self {
    self.render_objects.push(node_index);
    self
  }

  pub fn get_len(&self) -> usize {
    self.render_objects.len()
  }

  pub fn sort_for_opaque(&mut self, scene: &Scene) {
    self.render_objects.sort_unstable_by(|a, b| {
      let a_render_data = scene.get_node_render_data(*a);
      let b_render_data = scene.get_node_render_data(*b);

      // near to far;
      a_render_data
        .camera_distance
        .partial_cmp(&b_render_data.camera_distance)
        .unwrap()
    });
  }

  pub fn sort_for_transparent(&mut self, scene: &Scene) {
    self.render_objects.sort_unstable_by(|a, b| {
      let a_render_data = scene.get_node_render_data(*a);
      let b_render_data = scene.get_node_render_data(*b);

      // far to near;
      b_render_data
        .camera_distance
        .partial_cmp(&a_render_data.camera_distance)
        .unwrap()
    });
  }
}
