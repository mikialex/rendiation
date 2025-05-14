use rendiation_gizmo::*;

use crate::*;

pub fn widget_root(viewer_cx: &mut ViewerCx, f: impl FnOnce(&mut UI3dCx)) {
  // let widget_scene_handle = // todo
  // let scene = w.replace_target_scene(self.widget_scene);

  let mut cx = match &mut viewer_cx.stage {
    ViewerCxStage::EventHandling {
      reader,
      interaction,
      input,
      ..
    } => UI3dCx::new_event_stage(
      viewer_cx.memory,
      UIEventStageCx {
        platform_event: input,
        interaction_cx: interaction,
      },
      reader,
      viewer_cx.dyn_cx,
    ),
    ViewerCxStage::SceneContentUpdate { writer } => {
      // writer.scene =  // todo
      UI3dCx::new_update_stage(viewer_cx.memory, viewer_cx.dyn_cx, writer)
    }
  };

  f(&mut cx);

  if let ViewerCxStage::SceneContentUpdate { writer } = &mut viewer_cx.stage {
    // writer.scene =  // todo
  }
}

pub fn use_viewer_gizmo(cx: &mut ViewerCx) {
  // let state = cx.use_plain_state::<Option<GizmoControlTargetState>>();  todo
  let state: &mut Option<GizmoControlTargetState> = &mut None;
  let is_in_control: &mut bool = &mut false;
  let view_update: &mut Option<(EntityHandle<SceneNodeEntity>, GizmoUpdateTargetLocal)> = &mut None;

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
