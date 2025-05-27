use rendiation_gizmo::*;

use crate::*;

pub fn use_viewer_gizmo(cx: &mut ViewerCx) {
  let (cx, state) = cx.use_plain_state::<Option<GizmoControlTargetState>>();
  let (cx, view_update) =
    cx.use_plain_state::<Option<(EntityHandle<SceneNodeEntity>, GizmoUpdateTargetLocal)>>();

  let mut node = None;
  if let ViewerCxStage::EventHandling {
    reader, derived, ..
  } = &mut cx.stage
  {
    *state = cx.viewer.scene.selected_target.map(|target| {
      node = reader
        .scene_model
        .read_foreign_key::<SceneModelRefNode>(target);
      let node = node.unwrap();

      let target_local_mat = reader
        .node_reader
        .read::<SceneNodeLocalMatrixComponent>(node);

      let target_world_mat = derived.world_mat.access(&node).unwrap();
      let target_parent_world_mat =
        if let Some(parent) = reader.node_reader.read::<SceneNodeParentIdx>(node) {
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
  }

  widget_root(cx, |cx| {
    inject_cx(cx, state, |cx| {
      use_gizmo(cx);
    });
  });

  match &mut cx.stage {
    ViewerCxStage::EventHandling { .. } => {
      let cx = &mut cx.dyn_cx;
      *view_update = cx
        .message
        .take::<GizmoUpdateTargetLocal>()
        .map(|a| (node.unwrap(), a));

      if cx.message.take::<GizmoInControl>().is_some() {
        cx.message.put(CameraControlBlocked);
        cx.message.put(PickSceneBlocked);
      }
    }
    ViewerCxStage::SceneContentUpdate { writer, .. } => {
      if let Some((node, update)) = view_update.take() {
        writer.set_local_matrix(node, update.0);
      }
    }
    _ => {}
  }
}
