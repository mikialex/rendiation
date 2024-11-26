use rendiation_gizmo::{gizmo, GizmoControlTargetState};

use crate::*;

pub struct GizmoBridge {
  gizmo: Box<dyn Widget>,
}

impl GizmoBridge {
  pub fn new(w: &mut SceneWriter) -> Self {
    Self {
      gizmo: Box::new(gizmo(w)),
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

    cx.scoped_cx(&mut target, |cx| {
      self.gizmo.update_state(cx);
    });
  }

  fn update_view(&mut self, cx: &mut DynCx) {
    self.gizmo.update_view(cx);
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    self.gizmo.clean_up(cx);
  }
}
