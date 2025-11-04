use rendiation_gizmo::*;

use crate::*;

pub fn use_viewer_gizmo(cx: &mut UI3dCx, selected_model: Option<EntityHandle<SceneModelEntity>>) {
  let (cx, state) = cx.use_plain_state_default::<Option<GizmoControlTargetState>>();
  let (cx, view_update) =
    cx.use_plain_state_default::<Option<(EntityHandle<SceneNodeEntity>, GizmoUpdateTargetLocal)>>();

  let mut node = None;
  cx.on_event(|e, reader, _| {
    *state = selected_model.map(|target| {
      node = reader
        .scene_model
        .read_foreign_key::<SceneModelRefNode>(target);
      let node = node.unwrap();

      let target_local_mat = reader
        .node_reader
        .read::<SceneNodeLocalMatrixComponent>(node);

      let target_world_mat = e.widget_env.get_world_mat(node).unwrap();
      let target_parent_world_mat =
        if let Some(parent) = reader.node_reader.read::<SceneNodeParentIdx>(node) {
          let parent = unsafe { EntityHandle::from_raw(parent) };
          e.widget_env.get_world_mat(parent).unwrap()
        } else {
          Mat4::identity()
        };

      GizmoControlTargetState {
        target_local_mat,
        target_parent_world_mat,
        target_world_mat,
      }
    });
  });

  inject_cx(cx, state, |cx| {
    use_gizmo(cx);
  });

  cx.on_event(|_, _, cx| {
    *view_update = cx
      .message
      .take::<GizmoUpdateTargetLocal>()
      .map(|a| (node.unwrap(), a));

    if cx.message.take::<GizmoInControl>().is_some() {
      cx.message.put(CameraControlBlocked);
      cx.message.put(PickSceneBlocked);
    }
  });

  cx.on_update(|writer, _| {
    if let Some((node, update)) = view_update.take() {
      writer.set_local_matrix(node, update.0);
    }
  });
}
