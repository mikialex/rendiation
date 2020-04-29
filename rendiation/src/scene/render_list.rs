use super::scene::Scene;

pub struct RenderList {
  render_objects: Vec<usize>,
}

impl RenderList {
  pub fn new() -> Self {
    Self {
      render_objects: Vec::new(),
    }
  }

  pub fn get_len(&self) -> usize {
    self.render_objects.len()
  }

  pub fn clear(&mut self) {
    self.render_objects.clear();
  }

  pub fn sort_for_opaque(&mut self, scene: &Scene) {
    self.render_objects.sort_unstable_by(|a, b| {
      let (a_render_data, _) = scene.nodes_render_data.get_unknown_gen(*a).unwrap();
      let (b_render_data, _) = scene.nodes_render_data.get_unknown_gen(*b).unwrap();

      // near to far;
      a_render_data
        .camera_distance
        .partial_cmp(&b_render_data.camera_distance)
        .unwrap()
    });
  }

  pub fn sort_for_transparent(&mut self, scene: &Scene) {
    self.render_objects.sort_unstable_by(|a, b| {
      let (a_render_data, _) = scene.nodes_render_data.get_unknown_gen(*a).unwrap();
      let (b_render_data, _) = scene.nodes_render_data.get_unknown_gen(*b).unwrap();

      // far to near;
      b_render_data
        .camera_distance
        .partial_cmp(&a_render_data.camera_distance)
        .unwrap()
    });
  }
}
