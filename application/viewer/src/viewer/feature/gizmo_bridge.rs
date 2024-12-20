use rendiation_gizmo::*;

use crate::*;

pub struct GizmoBridge {
  gizmo: Box<dyn Widget>,
  widget_scene: EntityHandle<SceneEntity>,
  state: Option<GizmoControlTargetState>,
  is_in_control: bool,
  view_update: Option<(EntityHandle<SceneNodeEntity>, GizmoUpdateTargetLocal)>,
}

impl GizmoBridge {
  pub fn new(cx: &mut DynCx) -> Self {
    access_cx!(cx, viewer_scene, Viewer3dSceneCtx);
    let widget_scene = viewer_scene.widget_scene;
    access_cx_mut!(cx, w, SceneWriter);
    let gizmo = w.write_other_scene(widget_scene, |w| Box::new(gizmo(w)));
    Self {
      gizmo,
      state: None,
      view_update: None,
      is_in_control: false,
      widget_scene,
    }
  }
}

impl Widget for GizmoBridge {
  fn update_state(&mut self, cx: &mut DynCx) {
    access_cx!(cx, scene_cx, Viewer3dSceneCtx);
    access_cx!(cx, derived, Viewer3dSceneDerive);
    access_cx!(cx, scene_reader, SceneReader);

    let mut node = None;
    self.state = scene_cx.selected_target.map(|target| {
      node = scene_reader
        .scene_model
        .read_foreign_key::<SceneModelRefNode>(target);
      let node = node.unwrap();

      let target_local_mat = scene_reader
        .node_reader
        .read::<SceneNodeLocalMatrixComponent>(node);

      let target_world_mat = derived.world_mat.access(&node).unwrap();
      let target_parent_world_mat =
        if let Some(parent) = scene_reader.node_reader.read::<SceneNodeParentIdx>(node) {
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

    cx.scoped_cx(&mut self.state, |cx| {
      self.gizmo.update_state(cx);
      self.view_update = cx
        .message
        .take::<GizmoUpdateTargetLocal>()
        .map(|a| (node.unwrap(), a));
    });

    if cx.message.take::<GizmoInControl>().is_some() {
      self.is_in_control = true;
    }

    if cx.message.take::<GizmoOutControl>().is_some() {
      self.is_in_control = false;
    }

    if self.is_in_control {
      cx.message.put(CameraControlBlocked);
      cx.message.put(PickSceneBlocked);
    }
  }

  fn update_view(&mut self, cx: &mut DynCx) {
    access_cx_mut!(cx, w, SceneWriter);
    let scene = w.replace_target_scene(self.widget_scene);

    cx.scoped_cx(&mut self.state, |cx| {
      self.gizmo.update_view(cx);
    });

    if let Some((node, update)) = self.view_update.take() {
      access_cx_mut!(cx, w, SceneWriter);
      w.set_local_matrix(node, update.0);
    }

    access_cx_mut!(cx, w, SceneWriter);
    w.scene = scene;
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    access_cx_mut!(cx, w, SceneWriter);
    let scene = w.replace_target_scene(self.widget_scene);

    self.gizmo.clean_up(cx);

    access_cx_mut!(cx, w, SceneWriter);
    w.scene = scene;
  }
}
