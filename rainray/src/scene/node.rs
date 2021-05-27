use rendiation_algebra::*;

use crate::SceneNodePayload;

pub struct SceneNode {
  pub payloads: Vec<SceneNodePayload>,

  pub visible: bool,
  pub local_matrix: Mat4<f32>,

  pub net_visible: bool,
  pub world_matrix: Mat4<f32>,
}

impl Default for SceneNode {
  fn default() -> Self {
    Self {
      visible: true,
      local_matrix: Mat4::one(),
      payloads: Vec::new(),
      net_visible: true,
      world_matrix: Mat4::one(),
    }
  }
}

impl SceneNode {
  pub fn update(&mut self, parent: Option<&Self>) {
    if let Some(parent) = parent {
      self.net_visible = self.visible && parent.net_visible;
      if self.net_visible {
        self.world_matrix = parent.world_matrix * self.local_matrix;
      }
    } else {
      self.world_matrix = self.local_matrix;
      self.net_visible = self.visible
    }
  }
}
