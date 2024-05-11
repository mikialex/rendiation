use crate::*;

#[derive(Default)]
pub struct UIGroup {
  children: Vec<Box<dyn View>>,
}

impl UIGroup {
  pub fn with_child(mut self, child: impl View + 'static) -> Self {
    self.children.push(Box::new(child));
    self
  }
}

impl View for UIGroup {
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx) {
    for c in &mut self.children {
      c.update_view(cx)
    }
  }
  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx) {
    for c in &mut self.children {
      c.update_state(cx)
    }
  }
}

pub struct UINode {
  node: AllocIdx<SceneNodeEntity>,
  children: Vec<Box<dyn View>>,
}

impl UINode {
  pub fn node(&self) -> AllocIdx<SceneNodeEntity> {
    self.node
  }
  pub fn with_child<V: View + 'static>(
    mut self,
    child: impl FnOnce(AllocIdx<SceneNodeEntity>) -> V,
  ) -> Self {
    self.children.push(Box::new(child(self.node)));
    self
  }
}

impl Default for UINode {
  fn default() -> Self {
    todo!()
  }
}

impl View for UINode {
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx) {
    todo!()
  }

  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx) {
    todo!()
  }
}
