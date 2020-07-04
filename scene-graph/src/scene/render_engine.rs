use crate::{Culler, RenderList, Scene, SceneGraphBackend, SceneNode};

pub struct RenderEngine<T: SceneGraphBackend> {
  pub scene_raw_list: RenderList<T>,
  pub culled_list: RenderList<T>,
  pub culler: Culler,
}

impl<T: SceneGraphBackend> RenderEngine<T> {
  pub fn new() -> Self {
    Self {
      scene_raw_list: RenderList::new(),
      culled_list: RenderList::new(),
      culler: Culler::new(),
    }
  }

  pub fn update_render_list(&mut self, scene: &mut Scene<T>) {
    self.scene_raw_list.clear();
    let mut stack = Vec::new(); // todo, where should I reuse;
    scene.nodes.traverse(
      scene.get_root().handle(),
      &mut stack,
      |this: &mut SceneNode<T>, parent: Option<&mut SceneNode<T>>| {
        let this_handle = this.handle();
        let this_data = this.data_mut();
        if let Some(parent) = parent {
          let parent = parent.data();
          this_data.render_data.world_matrix =
            parent.render_data.world_matrix * this_data.render_data.local_matrix;
          this_data.net_visible = this_data.visible && parent.net_visible;
        }
        if !this_data.visible {
          return; // skip drawcall collect
        }

        this_data.render_objects.iter().for_each(|id| {
          self.scene_raw_list.push(this_handle, *id);
        });
      },
    );
  }

  pub fn execute_culling(&mut self, scene: &Scene<T>) {
    self.culled_list.clear();

    for drawcall in &self.scene_raw_list.drawcalls {
      if self.culler.test_is_visible(drawcall.node, scene) {
        self.culled_list.push_drawcall(*drawcall);
      }
    }
  }
}
