use crate::*;

pub struct UINode {
  node: EntityHandle<SceneNodeEntity>,
  children: WidgetGroup,
}

impl UINode {
  pub fn new(v: &mut SceneWriter) -> Self {
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
    v: &mut SceneWriter,
    child: impl FnOnce(EntityHandle<SceneNodeEntity>, &mut SceneWriter) -> V,
  ) -> Self {
    self.children = self.children.with_child(child(self.node, v));
    self
  }

  pub fn set_visible(&mut self, cx3d: &mut SceneWriter, v: bool) -> &mut Self {
    cx3d
      .node_writer
      .write::<SceneNodeVisibleComponent>(self.node, v);
    self
  }
  pub fn set_matrix(&mut self, cx3d: &mut SceneWriter, mat: Mat4<f32>) -> &mut Self {
    cx3d
      .node_writer
      .write::<SceneNodeLocalMatrixComponent>(self.node, mat);
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
    access_cx_mut!(cx, scene_cx, SceneWriter);
    scene_cx.node_writer.delete_entity(self.node);
  }
}
