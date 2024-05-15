use crate::*;

pub struct UINode {
  node: AllocIdx<SceneNodeEntity>,
  children: WidgetGroup,
}

impl UINode {
  pub fn new(v: &mut View3dProvider) -> Self {
    todo!()
  }
  pub fn node(&self) -> AllocIdx<SceneNodeEntity> {
    self.node
  }
  pub fn with_child<V: Widget + 'static>(
    mut self,
    v: &mut View3dProvider,
    child: impl FnOnce(AllocIdx<SceneNodeEntity>, &mut View3dProvider) -> V,
  ) -> Self {
    self.children = self.children.with_child(child(self.node, v));
    self
  }
}

impl Widget for UINode {
  fn update_view(&mut self, cx: &mut StateCx) {
    self.children.update_view(cx)
  }

  fn update_state(&mut self, cx: &mut StateCx) {
    self.children.update_state(cx)
  }

  fn clean_up(&mut self, cx: &mut StateCx) {
    todo!();
    self.children.clean_up(cx)
  }
}
