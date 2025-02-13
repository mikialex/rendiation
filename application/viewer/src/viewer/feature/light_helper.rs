use std::sync::Arc;

use fast_hash_collection::*;
use rendiation_mesh_core::AttributesMeshData;
use rendiation_mesh_generator::*;

use crate::*;

pub struct SceneSpotLightHelper {
  helper_models: FastHashMap<EntityHandle<SpotLightEntity>, UIWidgetModel>,

  world_mat: BoxedDynReactiveQuery<EntityHandle<SpotLightEntity>, Mat4<f32>>,
  half_cone_angle: BoxedDynReactiveQuery<EntityHandle<SpotLightEntity>, f32>,
  half_penumbra_angle: BoxedDynReactiveQuery<EntityHandle<SpotLightEntity>, f32>,
  cutoff: BoxedDynReactiveQuery<EntityHandle<SpotLightEntity>, f32>,

  pending_updates: Option<FastHashMap<EntityHandle<SpotLightEntity>, Option<AttributesMeshData>>>,
}

struct FindChangedKey<T> {
  changed_keys: FastHashMap<EntityHandle<T>, bool>,
}

impl<T> Default for FindChangedKey<T> {
  fn default() -> Self {
    Self {
      changed_keys: Default::default(),
    }
  }
}

impl<T> FindChangedKey<T> {
  pub fn merge_changed_keys(
    &mut self,
    changed_keys: &impl Query<Key = EntityHandle<T>>,
  ) -> &mut Self {
    self
  }
  pub fn iter_changed_keys(&self) -> impl Iterator<Item = &Option<EntityHandle<T>>> + '_ {
    self.changed_keys.iter()
  }
}

impl SceneSpotLightHelper {
  pub fn new(scene: EntityHandle<SceneEntity>) -> Self {
    let set = global_watch()
      .watch::<SpotLightRefScene>()
      .collective_filter(move |v| v.unwrap() == scene.into_raw())
      .collective_map(|_| {});

    // let changes = camera
    //   .filter_by_keyset(set)
    //   .collective_map(|t| t.view_projection_inv)
    //   .into_boxed();

    Self {
      helper_models: Default::default(),
      world_mat: todo!(),
      half_cone_angle: global_watch()
        .watch::<SpotLightHalfConeAngle>()
        .into_boxed(),
      half_penumbra_angle: global_watch()
        .watch::<SpotLightHalfPenumbraAngle>()
        .into_boxed(),
      cutoff: global_watch()
        .watch::<SpotLightCutOffDistance>()
        .into_boxed(),
      pending_updates: None,
    }
  }

  pub fn prepare_update(&mut self, cx: &mut Context) {
    let (mat_c, mat) = self.world_mat.poll_changes(cx);
    let (half_cone_angle_c, half_cone_angle) = self.half_cone_angle.poll_changes(cx);
    let (half_penumbra_angle_c, half_penumbra_angle) = self.half_penumbra_angle.poll_changes(cx);
    let (cutoff_c, cutoff) = self.cutoff.poll_changes(cx);

    self.pending_updates = FindChangedKey::default()
      .merge_changed_keys(&mat_c)
      .merge_changed_keys(&half_cone_angle_c)
      .merge_changed_keys(&half_penumbra_angle_c)
      .merge_changed_keys(&cutoff_c)
      .iter_changed_keys()
      .map(|k| {
        let mesh = k.map(|k| {
          create_debug_line_mesh(
            half_cone_angle.access(&k).unwrap(),
            half_penumbra_angle.access(&k).unwrap(),
            cutoff.access(&k).unwrap(),
          )
        });
        (*k, mesh)
      })
      .collect::<FastHashMap<_, _>>()
      .into();

    // self.pending_updates = changes.materialize().into()
  }

  pub fn apply_updates(
    &mut self,
    scene_cx: &mut SceneWriter,
    widget_target: EntityHandle<SceneEntity>,
    main_camera: EntityHandle<SceneCameraEntity>,
  ) {
    scene_cx.write_other_scene(widget_target, |scene_cx| {
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
    });

    if let Some(self_hidden_camera) = self.self_hidden_camera {
      if self_hidden_camera != main_camera {
        if let Some(helper) = self.helper_models.get_mut(&self_hidden_camera) {
          helper.set_visible(scene_cx, true);
        }
      }
    }
    self.self_hidden_camera = Some(main_camera);
    if let Some(helper) = self.helper_models.get_mut(&main_camera) {
      helper.set_visible(scene_cx, false);
    }
  }

  pub fn do_cleanup(&mut self, scene_cx: &mut SceneWriter) {
    self
      .helper_models
      .values_mut()
      .for_each(|m| m.do_cleanup(scene_cx));
  }
}

fn create_debug_line_mesh(half_angle: f32, half_penumbra: f32, cutoff: f32) -> AttributesMeshData {
  todo!()
}

fn create_circle(radius: f32) {
  UnitCircle.transform_by(Mat3::scale(Vec2::splat(radius)));
}
