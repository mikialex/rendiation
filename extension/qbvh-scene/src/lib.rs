use database::*;
use rendiation_geometry::*;
use rendiation_qbvh::*;

pub type SceneQbvh = Qbvh<u32, Box3ForSimd, SimdBox3>;

mod iter;
pub use iter::*;

pub fn use_scene_qbvh(
  cx: &mut impl DBHookCxLike,
  world_bounding: UseResult<impl DualQueryLike<Key = u32, Value = Box3<f64>>>,
) -> Option<LockReadGuardHolder<SceneQbvh>> {
  let (cx, bvh) = cx.use_sharable_plain_state(SceneQbvh::default);

  let bvh_ = bvh.clone();
  let _ = world_bounding.map_spawn_stage_in_thread_dual_query(cx, move |world_bounding| {
    let mut bvh = bvh_.write();
    let (view, delta) = world_bounding.view_delta();
    update_qbvh(&mut bvh, delta.into_change(), view);
  });

  cx.when_resolve_stage(|| bvh.make_read_holder())
}

fn update_qbvh(
  bvh: &mut SceneQbvh,
  world_bounding_changes: impl DataChanges<Key = u32, Value = Box3<f64>>,
  world_bounding_view: impl Query<Key = u32, Value = Box3<f64>>,
) {
  for k in world_bounding_changes.iter_removed() {
    debug_log!("remove with id: {k}");
    bvh.remove(k);
  }

  for (k, v) in world_bounding_changes.iter_update_or_insert() {
    if v.is_empty() {
      debug_log!("the bounding of item with id: {k} has been downgraded");
      bvh.remove(k);
    } else {
      debug_log!("pre update with id: {k}, bounding: {v:?}");
      bvh.pre_update_bounding_or_insert(k);
    }
  }

  bvh.refit_bounding(|leaf| {
    let bbox = world_bounding_view.access(&(*leaf).into()).unwrap();
    // todo, the current implementation not support large world precision.
    let bbox = Box3::new(bbox.min.into_f32(), bbox.max.into_f32());
    box3_to_box3_for_simd(bbox)
  });

  if ENABLE_SCENE_BVH_DEBUG {
    bvh.check_topology();
  }

  let mut work_space = QbvhUpdateWorkspace::<Box3ForSimd>::default();
  // todo: we should limit the frequency of rebalance.
  bvh.rebalance(&mut work_space, CenterDataSplitter::<3>::new(true));

  if ENABLE_SCENE_BVH_DEBUG {
    bvh.check_topology();
  }
}

pub const ENABLE_SCENE_BVH_LOGGING: bool = false;
#[macro_export]
macro_rules! debug_log {
  ($($e:expr),+) => {
    {
      if ENABLE_SCENE_BVH_LOGGING {
        log::info!($($e),+)
      }
    }
  };
}

pub const ENABLE_SCENE_BVH_DEBUG: bool = false;
