use super::scene::Scene;
use generational_arena::Index;

#[derive(Copy, Clone)]
pub struct Drawcall {
  pub render_object: Index,
  pub node: Index,
}

pub struct RenderList {
  pub drawcalls: Vec<Drawcall>,
}

impl RenderList {
  pub fn new() -> Self {
    Self {
      drawcalls: Vec::new(),
    }
  }

  pub fn clear(&mut self) -> &mut Self {
    self.drawcalls.clear();
    self
  }

  pub fn push_drawcall(&mut self, drawcall: Drawcall) -> &mut Self {
    self.drawcalls.push(drawcall);
    self
  }

  pub fn push(&mut self, node: Index, render_object: Index) -> &mut Self {
    self.drawcalls.push(Drawcall {
      render_object,
      node,
    });
    self
  }

  pub fn len(&self) -> usize {
    self.drawcalls.len()
  }

  pub fn sort_for_opaque(&mut self, scene: &Scene) {
    self.drawcalls.sort_unstable_by(|a, b| {
      let a_render_data = scene.get_node_render_data(a.node);
      let b_render_data = scene.get_node_render_data(b.node);

      // near to far;
      a_render_data
        .camera_distance
        .partial_cmp(&b_render_data.camera_distance)
        .unwrap()
    });
  }

  pub fn sort_for_transparent(&mut self, scene: &Scene) {
    self.drawcalls.sort_unstable_by(|a, b| {
      let a_render_data = scene.get_node_render_data(a.node);
      let b_render_data = scene.get_node_render_data(b.node);

      // far to near;
      b_render_data
        .camera_distance
        .partial_cmp(&a_render_data.camera_distance)
        .unwrap()
    });
  }
}
