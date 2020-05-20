use crate::{Culler, RenderList, Scene, SceneGraphBackEnd};

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

  pub fn execute_culling<T: SceneGraphBackEnd>(&mut self, scene: &Scene<T>) {
    self.culled_list.clear();

    for drawcall in &self.scene_raw_list.drawcalls {
      if self.culler.test_is_visible(drawcall.node, scene) {
        self.culled_list.push_drawcall(*drawcall);
      }
    }
  }
}
