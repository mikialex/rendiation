use database::*;
use rendiation_geometry::*;
use rendiation_qbvh::*;
use rendiation_scene_core::*;
use rendiation_scene_geometry_query::*;

pub type SceneQbvh = Qbvh<u32, Box3ForSimd, SimdBox3>;

mod iter;
pub use iter::*;

/// margin is necessary for line-like primitives
pub fn use_scene_qbvh(
  cx: &mut impl DBHookCxLike,
  world_bounding: UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = Box3<f64>>>,
  margin: UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = f32>>,
) -> Option<LockReadGuardHolder<SceneQbvh>> {
  let (cx, bvh) = cx.use_sharable_plain_state(SceneQbvh::default);

  let bvh_ = bvh.clone();

  let _ = world_bounding.join(margin).map_spawn_stage_in_thread(
    cx,
    |(w, m)| w.has_delta_hint() || m.has_delta_hint(),
    move |(world_bounding, margin)| {
      let mut bvh = bvh_.write();
      let (view, delta) = world_bounding.view_delta();
      let view = view.skip_generation_check::<SceneModelEntity>();
      let (m_view, m_delta) = margin.view_delta();
      let m_view = m_view.skip_generation_check::<SceneModelEntity>();
      update_qbvh(
        &mut bvh,
        // note: map_changes_key to convert handle to index is ok
        // same index add, remove will be correctly expressed as a change in index.
        delta.into_change().map_changes_key(|k| k.index()),
        |index| view.access(&index).unwrap(),
        m_delta.into_change().map_changes_key(|k| k.index()),
        // note: we here not require strict value scope for margin, the default margin is always 0.
        |index| m_view.access(&index).unwrap_or(0.),
      );
    },
  );

  cx.when_resolve_stage(|| bvh.make_read_holder())
}

fn update_qbvh(
  bvh: &mut SceneQbvh,
  world_bounding_changes: impl DataChanges<Key = u32, Value = Box3<f64>>,
  world_bounding_view: impl Fn(u32) -> Box3<f64>,
  margin_changes: impl DataChanges<Key = u32, Value = f32>,
  margin_view: impl Fn(u32) -> f32,
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
    let bbox = world_bounding_view(*leaf);
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

  // only handle change, removal has handled within bounding changes
  for (k, _c) in margin_changes.iter_update_or_insert() {
    bvh.pre_update_margin(k.alloc_index());
  }

  // No need to rebalance bvh tree, when only box margin is changed.
  bvh.refit_margin(|leaf| margin_view(*leaf));

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
