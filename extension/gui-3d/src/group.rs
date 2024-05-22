use crate::*;

pub struct UINode {
  node: EntityHandle<SceneNodeEntity>,
  children: WidgetGroup,
}

impl UINode {
  pub fn new(v: &mut Scene3dWriter) -> Self {
    Self {
      node: v.node_writer.new_entity(),
      children: Default::default(),
    }
  }
  pub fn node(&self) -> EntityHandle<SceneNodeEntity> {
    self.node
  }
  pub fn with_child<V: Widget + 'static>(
    mut self,
    v: &mut Scene3dWriter,
    child: impl FnOnce(EntityHandle<SceneNodeEntity>, &mut Scene3dWriter) -> V,
  ) -> Self {
    self.children = self.children.with_child(child(self.node, v));
    self
  }
}

impl Widget for UINode {
  fn update_view(&mut self, cx: &mut DynCx) {
    self.children.update_view(cx)
  }

  fn update_state(&mut self, cx: &mut DynCx) {
    self.children.update_state(cx)
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    access_cx_mut!(cx, scene_cx, Scene3dWriter);
    scene_cx.node_writer.delete_entity(self.node);
  }
}
