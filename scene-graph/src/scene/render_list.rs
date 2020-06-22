use super::scene::Scene;
use crate::{RenderObjectHandle, SceneGraphBackend, SceneNode};
use arena::Handle;

#[derive(Copy, Clone)]
pub struct Drawcall<T: SceneGraphBackend> {
  pub render_object: RenderObjectHandle<T>,
  pub node: Handle<SceneNode>,
}

pub struct RenderList<T: SceneGraphBackend> {
  pub drawcalls: Vec<Drawcall<T>>,
}

impl<T: SceneGraphBackend> RenderList<T> {
  pub fn new() -> Self {
    Self {
      drawcalls: Vec::new(),
    }
  }

  pub fn clear(&mut self) -> &mut Self {
    self.drawcalls.clear();
    self
  }

  pub fn push_drawcall(&mut self, drawcall: Drawcall<T>) -> &mut Self {
    self.drawcalls.push(drawcall);
    self
  }

  pub fn push(
    &mut self,
    node: Handle<SceneNode>,
    render_object: RenderObjectHandle<T>,
  ) -> &mut Self {
    self.drawcalls.push(Drawcall {
      render_object,
      node,
    });
    self
  }

  pub fn len(&self) -> usize {
    self.drawcalls.len()
  }

  pub fn sort_for_opaque(&mut self, scene: &Scene<T>) {
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

  pub fn sort_for_transparent(&mut self, scene: &Scene<T>) {
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
