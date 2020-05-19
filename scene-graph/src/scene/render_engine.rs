use crate::{Culler, RenderList, Scene, SceneNode};
use rendiation::{RenderTargetAble, WGPURenderer};

pub struct SceneGraphRenderEngine {
  scene_raw_list: RenderList,
  culled_list: RenderList,
  culler: Culler,
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

  pub fn render(
    &mut self,
    scene: &mut Scene,
    renderer: &mut WGPURenderer,
    target: &impl RenderTargetAble,
  ) {
    self.scene_raw_list.clear();
    scene.traverse(
      scene.get_root().self_id,
      |this: &mut SceneNode, parent: Option<&mut SceneNode>| {
        this.render_objects.iter().for_each(|id| {
          self.scene_raw_list.push(this.get_id(), *id);
        });
      },
    );

    scene
      .background
      .render(renderer, target.create_render_pass_builder());

    let mut pass = target
      .create_render_pass_builder()
      .first_color(|c| c.load_with_clear((0.1, 0.2, 0.3).into(), 1.0).ok())
      .create(&mut renderer.encoder);

    for drawcall in &self.scene_raw_list.drawcalls {
      // let node = self.nodes.get(drawcall.node).unwrap();
      let render_obj = scene.render_objects.get(drawcall.render_object).unwrap();
      render_obj.render(&mut pass, scene);
    }
  }
}
