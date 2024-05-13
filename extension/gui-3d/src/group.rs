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
  fn clean_up(&mut self, cx: &mut StateStore) {
    for child in &mut self.children{
      child.clean_up(cx)
    }
  }
}

pub struct UINode {
  node: AllocIdx<SceneNodeEntity>,
  children: UIGroup,
}

impl UINode {
  pub fn new(v: &mut View3dProvider) -> Self {
    todo!()
  }
  pub fn node(&self) -> AllocIdx<SceneNodeEntity> {
    self.node
  }
  pub fn with_child<V: View + 'static>(
    mut self,
    v: &mut View3dProvider,
    child: impl FnOnce(AllocIdx<SceneNodeEntity>, &mut View3dProvider) -> V,
  ) -> Self {
    self.children = self.children.with_child(child(self.node, v));
    self
  }
}

impl View for UINode {
  fn update_view(&mut self, cx: &mut View3dViewUpdateCtx) {
    self.children.update_view(cx)
  }

  fn update_state(&mut self, cx: &mut View3dStateUpdateCtx) {
    self.children.update_state(cx)
  }

  fn clean_up(&mut self, cx: &mut StateStore) {
    todo!();
    self.children.clean_up(cx)
  }
}
