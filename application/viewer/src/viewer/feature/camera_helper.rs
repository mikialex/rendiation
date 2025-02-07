use std::sync::Arc;

use fast_hash_collection::FastHashMap;
use rendiation_mesh_core::{AttributeSemantic, AttributesMeshData};

use crate::*;

/// query all camera in scene and maintain the helper models in scene
pub struct SceneCameraHelper {
  helper_models: FastHashMap<EntityHandle<SceneCameraEntity>, UIWidgetModel>,
  camera_changes: BoxedDynReactiveQuery<EntityHandle<SceneCameraEntity>, Mat4<f32>>,
  pending_updates:
    Option<Arc<FastHashMap<EntityHandle<SceneCameraEntity>, ValueChange<Mat4<f32>>>>>,
}

impl SceneCameraHelper {
  pub fn new(
    scene: EntityHandle<SceneEntity>,
    camera: impl ReactiveQuery<Key = EntityHandle<SceneCameraEntity>, Value = CameraTransform>,
  ) -> Self {
    let camera_set = global_watch()
      .watch::<SceneCameraBelongsToScene>()
      .collective_filter(move |v| v.unwrap() == scene.into_raw())
      .collective_map(|_| {});

    let camera_changes = camera
      .filter_by_keyset(camera_set)
      .collective_map(|t| t.view_projection)
      .into_boxed();

    Self {
      helper_models: Default::default(),
      camera_changes,
      pending_updates: None,
    }
  }

  pub fn prepare_update(&mut self, cx: &mut Context) {
    let (changes, _) = self.camera_changes.poll_changes(cx);
    self.pending_updates = changes.materialize().into()
  }

  pub fn apply_updates(&mut self, scene_cx: &mut SceneWriter) {
    if let Some(changes) = self.pending_updates.take() {
      for (k, c) in changes.iter_key_value() {
        match c {
          ValueChange::Remove(_) => {
            let mut model = self.helper_models.remove(&k).unwrap();
            model.do_cleanup(scene_cx);
          }
          ValueChange::Delta(new, _) => {
            let new_mesh = build_debug_line_in_camera_space(new);
            if let Some(helper) = self.helper_models.get_mut(&k) {
              helper.replace_new_shape_and_cleanup_old(scene_cx, new_mesh);
            } else {
              self
                .helper_models
                .insert(k, UIWidgetModel::new(scene_cx, new_mesh));
            }
          }
        }
      }
    }
  }

  pub fn do_cleanup(&mut self, scene_cx: &mut SceneWriter) {
    self
      .helper_models
      .values_mut()
      .for_each(|m| m.do_cleanup(scene_cx));
  }
}

fn build_debug_line_in_camera_space(project_mat: Mat4<f32>) -> AttributesMeshData {
  let zero = 0.0001;
  let one = 0.9999;

  let near = zero;
  let far = one;
  let left = -one;
  let right = one;
  let top = one;
  let bottom = -one;

  let min = Vec3::new(near, left, bottom);
  let max = Vec3::new(far, right, top);

  let lines: Vec<_> = line_box(min, max)
    .into_iter()
    .map(|[a, b]| [project_mat * a, project_mat * b])
    .collect();
  let lines: &[u8] = cast_slice(lines.as_slice());

  AttributesMeshData {
    attributes: vec![(AttributeSemantic::Positions, lines.to_vec())],
    indices: None,
    mode: rendiation_mesh_core::PrimitiveTopology::LineList,
    groups: Default::default(),
  }
}

fn line_box(min: Vec3<f32>, max: Vec3<f32>) -> impl IntoIterator<Item = [Vec3<f32>; 2]> {
  let near = min.x;
  let far = max.x;
  let left = min.z;
  let right = max.z;
  let top = max.y;
  let bottom = min.y;

  let near_left_down = Vec3::new(left, bottom, near);
  let near_left_top = Vec3::new(left, top, near);
  let near_right_down = Vec3::new(right, bottom, near);
  let near_right_top = Vec3::new(right, top, near);

  let far_left_down = Vec3::new(left, bottom, far);
  let far_left_top = Vec3::new(left, top, far);
  let far_right_down = Vec3::new(right, bottom, far);
  let far_right_top = Vec3::new(right, top, far);

  [
    [near_left_down, near_left_top],
    [near_right_down, near_right_top],
    [near_left_down, near_right_down],
    [near_left_top, near_right_top],
    //
    [far_left_down, far_left_top],
    [far_right_down, far_right_top],
    [far_left_down, far_right_down],
    [far_left_top, far_right_top],
    //
    [near_left_down, far_left_down],
    [near_left_top, far_left_top],
    [near_right_down, far_right_down],
    [near_right_top, far_right_top],
  ]
}
