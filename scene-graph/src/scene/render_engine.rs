use crate::{Culler, RenderList, Scene};

pub struct SceneGraphRenderEngine {
  pub scene_raw_list: RenderList,
  pub culled_list: RenderList,
  pub culler: Culler,
}

impl SceneGraphRenderEngine {
  pub fn new() -> Self {
    Self {
      scene_raw_list: RenderList::new(),
      culled_list: RenderList::new(),
      culler: Culler::new(),
    }
  }

  pub fn execute_culling(&mut self, scene: &Scene) {
    self.culled_list.clear();

    for drawcall in &self.scene_raw_list.drawcalls {
      if self.culler.test_is_visible(drawcall.node, scene) {
        self.culled_list.push_drawcall(*drawcall);
      }
    }
  }
}
