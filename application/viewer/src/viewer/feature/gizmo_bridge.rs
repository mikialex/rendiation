use rendiation_gizmo::*;

use crate::*;

pub struct GizmoBridge {
  gizmo: Box<dyn Widget>,
  view_update: Option<(EntityHandle<SceneNodeEntity>, GizmoUpdateTargetLocal)>,
}

impl GizmoBridge {
  pub fn new(w: &mut SceneWriter) -> Self {
    Self {
      gizmo: Box::new(gizmo(w)),
      view_update: None,
    }
  }
}

impl Widget for GizmoBridge {
  fn update_state(&mut self, cx: &mut DynCx) {
    access_cx!(cx, scene_cx, Viewer3dSceneCtx);
    access_cx!(cx, derived, Viewer3dSceneDerive);

    let node_view = global_entity_component_of::<SceneModelRefNode>().read_foreign_key();
    let node_local_mat_view = global_entity_component_of::<SceneNodeLocalMatrixComponent>().read();
    let node_parent = global_entity_component_of::<SceneNodeParentIdx>().read();

    let mut target = scene_cx.selected_target.map(|target| {
      let node = node_view.get(target).unwrap();
      let target_local_mat = node_local_mat_view.get_value(node).unwrap();
      let target_world_mat = derived.world_mat.access(&node).unwrap();
      let target_parent_world_mat = if let Some(parent) = node_parent.get_value(node).unwrap() {
        let parent = unsafe { EntityHandle::from_raw(parent) };
        derived.world_mat.access(&parent).unwrap()
      } else {
        Mat4::identity()
      };

      GizmoControlTargetState {
        target_local_mat,
        target_parent_world_mat,
        target_world_mat,
      }
    });

    let node = scene_cx
      .selected_target
      .map(|target| node_view.get(target).unwrap());

    cx.scoped_cx(&mut target, |cx| {
      self.gizmo.update_state(cx);
      self.view_update = cx
        .message
        .take::<GizmoUpdateTargetLocal>()
        .map(|a| (node.unwrap(), a));
    });
  }

  fn update_view(&mut self, cx: &mut DynCx) {
    self.gizmo.update_view(cx);
    if let Some((node, update)) = self.view_update.take() {
      access_cx_mut!(cx, w, SceneWriter);
      dbg!(update);
      w.set_local_matrix(node, update.0);
    }
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    self.gizmo.clean_up(cx);
  }
}
