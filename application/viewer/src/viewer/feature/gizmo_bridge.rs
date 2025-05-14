use rendiation_gizmo::*;

use crate::*;

pub fn widget_root(viewer_cx: &mut ViewerCx, f: impl FnOnce(&mut UI3dCx)) {
  let mut cx = match &mut viewer_cx.stage {
    ViewerCxStage::EventHandling {
      reader,
      interaction,
      input,
      widget_cx,
      ..
    } => UI3dCx::new_event_stage(
      viewer_cx.memory,
      UIEventStageCx {
        platform_event: input,
        interaction_cx: interaction,
        widget_env: *widget_cx,
      },
      reader,
      viewer_cx.dyn_cx,
      &mut viewer_cx.viewer.widget_intersection_group,
    ),
    ViewerCxStage::SceneContentUpdate { writer } => UI3dCx::new_update_stage(
      viewer_cx.memory,
      viewer_cx.dyn_cx,
      writer,
      &mut viewer_cx.viewer.widget_intersection_group,
    ),
  };

  let (cx, widget_scene_handle) = cx.use_state_init(|cx| cx.writer.scene_writer.new_entity());

  let mut scene_old = None;

  cx.on_update(|w, _| {
    scene_old = w.replace_target_scene(*widget_scene_handle).into();
  });

  f(cx);

  if let ViewerCxStage::SceneContentUpdate { writer } = &mut viewer_cx.stage {
    if let Some(scene) = scene_old.take() {
      writer.scene = scene
    }
  }
}

pub fn use_viewer_gizmo(cx: &mut ViewerCx) {
  let (cx, state) = cx.use_plain_state::<Option<GizmoControlTargetState>>();
  let (cx, is_in_control) = cx.use_plain_state::<bool>();
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
    use_gizmo(cx);
  });

  match &mut cx.stage {
    ViewerCxStage::EventHandling { .. } => {
      let cx = &mut cx.dyn_cx;
      *view_update = cx
        .message
        .take::<GizmoUpdateTargetLocal>()
        .map(|a| (node.unwrap(), a));

      if cx.message.take::<GizmoInControl>().is_some() {
        *is_in_control = true;
      }

      if cx.message.take::<GizmoOutControl>().is_some() {
        *is_in_control = false;
      }

      if *is_in_control {
        cx.message.put(CameraControlBlocked);
        cx.message.put(PickSceneBlocked);
      }
    }
    ViewerCxStage::SceneContentUpdate { writer } => {
      if let Some((node, update)) = view_update.take() {
        writer.set_local_matrix(node, update.0);
      }
    }
  }
}
