use std::ops::{Deref, DerefMut};

use rendiation_view_override_model::ViewAutoScalable;

use crate::*;

/// this controller is to extend the widget to support view independent
///
/// view config must be provided in the cx
pub struct ViewIndependentWidgetModel {
  model: UIWidgetModel,
  origin_local_mat: Mat4<f32>,
  local_mat_to_sync: Option<Mat4<f32>>,
}

impl Widget for ViewIndependentWidgetModel {
  fn update_state(&mut self, cx: &mut DynCx) {
    self.model.update_state(cx);
    access_cx!(cx, config, ViewIndependentComputer);
    access_cx!(cx, world_mat_access, Box<dyn WidgetEnvAccess>);
    access_cx!(cx, reader, SceneReader);
    let parent_world =
      if let Some(parent_node) = reader.node_reader.read::<SceneNodeParentIdx>(self.node) {
        let parent_node = unsafe { EntityHandle::from_raw(parent_node) };
        world_mat_access.get_world_mat(parent_node).unwrap()
      } else {
        Mat4::identity()
      };

    let origin_world = parent_world * self.origin_local_mat;
    let override_world_mat = config.scale.override_mat(
      origin_world,
      config.override_position,
      config.camera_world,
      config.camera_view_height,
      config.camera_proj,
    );

    self.local_mat_to_sync = Some(parent_world.inverse_or_identity() * override_world_mat);
  }

  fn update_view(&mut self, cx: &mut DynCx) {
    self.model.update_view(cx);
    access_cx_mut!(cx, writer, SceneWriter);
    if let Some(mat) = self.local_mat_to_sync.take() {
      writer.set_local_matrix(self.node, mat);
    }
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    self.model.clean_up(cx);
  }
}

impl Deref for ViewIndependentWidgetModel {
  type Target = UIWidgetModel;

  fn deref(&self) -> &Self::Target {
    &self.model
  }
}
impl DerefMut for ViewIndependentWidgetModel {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.model
  }
}

pub struct ViewIndependentComputer {
  override_position: Vec3<f32>,
  scale: ViewAutoScalable,
  camera_world: Mat4<f32>,
  camera_view_height: f32,
  camera_proj: PerspectiveProjection<f32>,
}

pub struct ViewIndependentRoot {
  node: UINode,
  config: ViewAutoScalable,
}

impl Deref for ViewIndependentRoot {
  type Target = UINode;

  fn deref(&self) -> &Self::Target {
    &self.node
  }
}
impl DerefMut for ViewIndependentRoot {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.node
  }
}

impl Widget for ViewIndependentRoot {
  fn update_state(&mut self, cx: &mut DynCx) {
    access_cx!(cx, access, Box<dyn WidgetEnvAccess>);

    let mut computer = ViewIndependentComputer {
      override_position: access.get_world_mat(self.node()).unwrap().position(),
      scale: self.config,
      camera_world: access.get_camera_world_mat(),
      camera_view_height: access.get_view_resolution().y as f32,
      camera_proj: access.get_camera_perspective_proj(),
    };

    cx.scoped_cx(&mut computer, |cx| self.node.update_state(cx));
  }

  fn update_view(&mut self, cx: &mut DynCx) {
    self.node.update_view(cx)
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    self.node.clean_up(cx)
  }
}

impl UINode {
  pub fn into_view_independent_root(self, independent_scale_factor: f32) -> ViewIndependentRoot {
    ViewIndependentRoot {
      node: self,
      config: ViewAutoScalable {
        independent_scale_factor,
      },
    }
  }
}

impl UIWidgetModel {
  pub fn into_view_independent(self, origin_local_mat: Mat4<f32>) -> ViewIndependentWidgetModel {
    ViewIndependentWidgetModel {
      model: self,
      local_mat_to_sync: None,
      origin_local_mat,
    }
  }
}
