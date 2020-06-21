use crate::{Culler, RenderList, Scene, SceneGraphBackend, SceneNode};

pub struct RenderEngine {
  pub scene_raw_list: RenderList,
  pub culled_list: RenderList,
  pub culler: Culler,
}

impl RenderEngine {
  pub fn new() -> Self {
    Self {
      scene_raw_list: RenderList::new(),
      culled_list: RenderList::new(),
      culler: Culler::new(),
    }
  }

  pub fn update_render_list<T: SceneGraphBackend>(&mut self, scene: &mut Scene<T>) {
    self.scene_raw_list.clear();
    let mut stack = Vec::new(); // todo
    scene.traverse(
      scene.get_root().self_id,
      &mut stack,
      |this: &mut SceneNode, parent: Option<&mut SceneNode>| {
        if let Some(parent) = parent {
          this.render_data.world_matrix =
            parent.render_data.world_matrix * this.render_data.local_matrix;
          this.net_visible = this.visible && parent.net_visible;
        }
        if !this.visible {
          return; // skip drawcall collect
        }

        this.render_objects.iter().for_each(|id| {
          self.scene_raw_list.push(this.get_id(), *id);
        });
      },
    );
  }

  pub fn execute_culling<T: SceneGraphBackend>(&mut self, scene: &Scene<T>) {
    self.culled_list.clear();

    for drawcall in &self.scene_raw_list.drawcalls {
      if self.culler.test_is_visible(drawcall.node, scene) {
        self.culled_list.push_drawcall(*drawcall);
      }
    }
  }
}
