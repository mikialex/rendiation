use rendiation_gizmo::*;

use crate::*;

pub struct GizmoBridge {
  gizmo: Box<dyn Widget>,
  widget_scene: EntityHandle<SceneEntity>,
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
      view_update: None,
      widget_scene,
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
    access_cx_mut!(cx, w, SceneWriter);
    let scene = w.replace_target_scene(self.widget_scene);

    self.gizmo.update_view(cx);

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
