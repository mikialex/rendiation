use crate::scene_graph::*;
use rendiation_render_entity::*;

pub struct RenderItem {
  pub render_object_index: usize,
  pub scene_node_index: usize,
  pub camera_distance: f32,
}

#[derive(Default)]
pub struct RenderList {
  renderables: Vec<RenderItem>,
}

impl RenderList {
  pub fn new() -> Self {
    RenderList {
      renderables: Vec::new(),
    }
  }

  pub fn get_len(&self) -> usize {
    self.renderables.len()
  }

  pub fn reset(&mut self) -> &mut Self {
    self.renderables.clear();
    self
  }

  pub fn add_renderable<Renderer>(
    &mut self,
    obj: &RenderObject<Renderer>,
    scene_node: &SceneNode,
    camera_distance: f32,
  ) -> &mut Self {
    self.renderables.push(RenderItem {
      render_object_index: obj.index,
      scene_node_index: scene_node.get_index(),
      camera_distance,
    });
    self
  }

  pub fn foreach<T>(&self, visitor: T)
  where
    T: FnMut(&RenderItem),
  {
    self.renderables.iter().for_each(visitor);
  }

  pub fn sort(&mut self) {
    self
      .renderables
      .sort_unstable_by(|a, b| a.camera_distance.partial_cmp(&b.camera_distance).unwrap());
  }
}
