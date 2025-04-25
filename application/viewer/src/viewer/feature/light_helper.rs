use fast_hash_collection::*;
use rendiation_mesh_core::{AttributeSemantic, AttributesMeshData};
use rendiation_mesh_generator::*;

use crate::*;

pub struct SceneSpotLightHelper {
  helper_models: FastHashMap<EntityHandle<SpotLightEntity>, UIWidgetModel>,

  world_mat: BoxedDynReactiveQuery<EntityHandle<SpotLightEntity>, Mat4<f32>>,
  half_cone_angle: BoxedDynReactiveQuery<EntityHandle<SpotLightEntity>, f32>,
  half_penumbra_angle: BoxedDynReactiveQuery<EntityHandle<SpotLightEntity>, f32>,
  cutoff: BoxedDynReactiveQuery<EntityHandle<SpotLightEntity>, f32>,

  pending_updates: FastHashMap<EntityHandle<SpotLightEntity>, AttributesMeshData>,
  pending_remove: FastHashSet<EntityHandle<SpotLightEntity>>,
}

struct FindChangedKey<T> {
  changed_keys: FastHashSet<EntityHandle<T>>,
}

impl<T> Default for FindChangedKey<T> {
  fn default() -> Self {
    Self {
      changed_keys: Default::default(),
    }
  }
}

impl<T> FindChangedKey<T> {
  pub fn merge_changed_keys<V>(
    &mut self,
    changed_keys: &impl Query<Key = EntityHandle<T>, Value = ValueChange<V>>,
  ) -> &mut Self {
    for (k, v) in changed_keys.iter_key_value() {
      if !v.is_removed() {
        self.changed_keys.insert(k);
      }
    }
    self
  }
  pub fn iter_changed_keys(&self) -> impl Iterator<Item = &EntityHandle<T>> + '_ {
    self.changed_keys.iter()
  }
}

impl SceneSpotLightHelper {
  pub fn new(
    _scene: EntityHandle<SceneEntity>,
    world_mats: BoxedDynReactiveQuery<EntityHandle<SceneNodeEntity>, Mat4<f32>>,
  ) -> Self {
    // let set = global_watch()
    //   .watch::<SpotLightRefScene>()
    //   .collective_filter(move |v| v.unwrap() == scene.into_raw())
    //   .collective_map(|_| {});

    let world_mat = world_mats
      .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SpotLightRefNode>())
      .into_boxed();

    Self {
      helper_models: Default::default(),
      world_mat,
      half_cone_angle: global_watch()
        .watch::<SpotLightHalfConeAngle>()
        .into_boxed(),
      half_penumbra_angle: global_watch()
        .watch::<SpotLightHalfPenumbraAngle>()
        .into_boxed(),
      cutoff: global_watch()
        .watch::<SpotLightCutOffDistance>()
        .into_boxed(),
      pending_updates: Default::default(),
      pending_remove: Default::default(),
    }
  }

  pub fn prepare_update(&mut self, cx: &mut Context) {
    let (mat_c, mat) = self.world_mat.describe(cx).resolve_kept();
    let (half_cone_angle_c, half_cone_angle) = self.half_cone_angle.describe(cx).resolve_kept();
    let (half_penumbra_angle_c, half_penumbra_angle) =
      self.half_penumbra_angle.describe(cx).resolve_kept();
    let (cutoff_c, cutoff) = self.cutoff.describe(cx).resolve_kept();

    self.pending_updates = FindChangedKey::default()
      .merge_changed_keys(&mat_c)
      .merge_changed_keys(&half_cone_angle_c)
      .merge_changed_keys(&half_penumbra_angle_c)
      .merge_changed_keys(&cutoff_c)
      .iter_changed_keys()
      .map(|k| {
        let mesh = create_debug_line_mesh(
          half_cone_angle.access(k).unwrap(),
          half_penumbra_angle.access(k).unwrap(),
          cutoff.access(k).unwrap(),
          mat.access(k).unwrap(),
        );
        (*k, mesh)
      })
      .collect::<FastHashMap<_, _>>();
  }

  pub fn apply_updates(
    &mut self,
    scene_cx: &mut SceneWriter,
    widget_target: EntityHandle<SceneEntity>,
  ) {
    scene_cx.write_other_scene(widget_target, |scene_cx| {
      for k in std::mem::take(&mut self.pending_remove) {
        let mut model = self.helper_models.remove(&k).unwrap();
        model.do_cleanup(scene_cx);
      }

      for (k, new_mesh) in std::mem::take(&mut self.pending_updates) {
        if let Some(helper) = self.helper_models.get_mut(&k) {
          helper.replace_new_shape_and_cleanup_old(scene_cx, new_mesh);
        } else {
          self
            .helper_models
            .insert(k, UIWidgetModel::new(scene_cx, new_mesh));
        }
      }
    });
  }

  pub fn do_cleanup(&mut self, scene_cx: &mut SceneWriter) {
    self
      .helper_models
      .values_mut()
      .for_each(|m| m.do_cleanup(scene_cx));
  }
}

fn create_debug_line_mesh(
  half_angle: f32,
  half_penumbra: f32,
  cutoff: f32,
  world_mat: Mat4<f32>,
) -> AttributesMeshData {
  let mut lines: Vec<_> = Default::default();

  fn build_cone(
    half_angle: f32,
    cutoff: f32,
    world_mat: Mat4<f32>,
    lines: &mut Vec<[Vec3<f32>; 2]>,
  ) {
    let radius = half_angle.tan() * cutoff;
    let angle_outlines_ends = [
      Vec3::new(-radius, 0., -cutoff),
      Vec3::new(radius, 0., -cutoff),
      Vec3::new(0., -radius, -cutoff),
      Vec3::new(0., radius, -cutoff),
    ];

    lines.extend(
      angle_outlines_ends
        .into_iter()
        .map(|ends| [world_mat.position(), world_mat * ends]),
    );

    let circle = create_circle(radius, cutoff).transform3d_by(world_mat);

    let step_count = 32;
    let step_size = 1.0 / step_count as f32;
    for i in 0..step_count {
      let start = circle.position(step_size * i as f32);
      let end = circle.position(step_size * (i + 1) as f32);
      lines.push([start, end]);
    }
  }

  build_cone(half_angle, cutoff, world_mat, &mut lines);
  build_cone(half_penumbra, cutoff, world_mat, &mut lines);

  let lines: &[u8] = cast_slice(lines.as_slice());

  AttributesMeshData {
    attributes: vec![(AttributeSemantic::Positions, lines.to_vec())],
    indices: None,
    mode: rendiation_mesh_core::PrimitiveTopology::LineList,
    groups: Default::default(),
  }
}

fn create_circle(radius: f32, offset: f32) -> impl ParametricCurve3D {
  UnitCircle
    .transform_by(Mat3::scale(Vec2::splat(radius)))
    .embed_to_surface(ParametricPlane.transform3d_by(Mat4::translate((0., 0., -offset))))
}
