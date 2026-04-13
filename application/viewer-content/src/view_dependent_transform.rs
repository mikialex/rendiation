use crate::*;

pub struct SceneModelViewDependentTransformOccShare(
  pub ViewerNDC,
  pub Arc<FastHashMap<ViewKey, (RawEntityHandle, Vec2<f32>)>>,
);

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for SceneModelViewDependentTransformOccShare {
  share_provider_hash_type_id! {}

  type Result = BoxedDynDualQuery<ViewSceneModelKey, Mat4<f64>>;

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let view_source = use_compute_incremental_source_by_diffing(cx, &self.1);
    let camera_transforms = cx.use_shared_dual_query(GlobalCameraTransformShare(self.0.clone()));

    use_occ_style_view_dependent_transform_data(cx, view_source, camera_transforms)
  }
}

/// compute change directly from full view, for good performance, the map should be small
pub fn use_compute_incremental_source_by_diffing<K: CKey, V: CValue>(
  cx: &mut impl QueryHookCxLike,
  new_state: &FastHashMap<K, V>,
) -> UseResult<BoxedDynDualQuery<K, V>> {
  let target = cx.use_shared_hash_map::<K, V>("full_to_incremental_source");

  match cx.stage() {
    QueryHookStage::SpawnTask { .. } => {
      let mut changes = FastHashMap::default();
      let mut target_ = target.write();
      let target__ = &mut *target_;

      let mut to_remove = Vec::new();
      for k in target__.keys() {
        if !new_state.contains_key(k) {
          to_remove.push(k.clone());
        }
      }

      let mut collector = QueryMutationCollector {
        delta: &mut changes,
        target: target__,
      };

      for k in to_remove {
        collector.remove(k.clone());
      }

      for (k, v) in new_state {
        collector.set_value(k.clone(), v.clone());
      }

      drop(target_);

      let r = DualQuery {
        view: target.make_read_holder(),
        delta: Arc::new(changes),
      }
      .into_boxed();
      UseResult::SpawnStageReady(r)
    }
    QueryHookStage::ResolveTask { .. } => UseResult::NotInStage,
    QueryHookStage::Other => UseResult::NotInStage,
  }
}
