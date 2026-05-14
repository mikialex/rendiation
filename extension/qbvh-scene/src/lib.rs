use database::*;
use fast_hash_collection::FastHashMap;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_qbvh::*;
use rendiation_scene_core::*;
use rendiation_scene_geometry_query::*;

pub type SceneQbvhImpl = Qbvh<u32, Box3ForSimd, SimdBox3>;

mod iter;
pub use iter::*;

#[derive(Default)]
pub struct SceneQbvh {
  internal: FastHashMap<RawEntityHandle, (SceneQbvhImpl, bool)>,
}

impl SceneQbvh {
  pub fn get_qbvh(&self, scene: RawEntityHandle) -> Option<&SceneQbvhImpl> {
    self.internal.get(&scene).map(|v| &v.0)
  }
  pub fn get_or_create_qbvh(&mut self, scene: RawEntityHandle) -> &mut SceneQbvhImpl {
    let bvh = self
      .internal
      .entry(scene)
      .or_insert_with(|| (Default::default(), true));
    bvh.1 = true;
    &mut bvh.0
  }

  pub fn flush_changed_bvh(&mut self, mut f: impl FnMut(&mut SceneQbvhImpl)) {
    for (bvh, changed) in self.internal.values_mut() {
      if *changed {
        *changed = false;
        f(bvh);
      }
    }
  }
}

pub fn generate_qbvh_wireframe(
  qbvh: &SceneQbvhImpl,
) -> Vec<Vec<(Vec3<f32>, Vec3<f32>)>> {
  let nodes = qbvh.nodes();
  if nodes.is_empty() {
    return Vec::new();
  }

  let mut depth_lines: Vec<Vec<(Vec3<f32>, Vec3<f32>)>> = Vec::new();
  let mut stack: Vec<(u32, usize)> = vec![(0, 0)];

  while let Some((node_idx, depth)) = stack.pop() {
    let node = &nodes[node_idx as usize];

    if depth >= depth_lines.len() {
      depth_lines.resize_with(depth + 1, Vec::new);
    }

    for lane in 0..QBVH_SIMD_WIDTH {
      let child = node.children[lane];
      if child == u32::MAX {
        continue;
      }
      let aabb_simd: Box3ForSimd = node.simd_aabb.extract(lane);
      let min: Vec3<f32> = aabb_simd.min.into();
      let max: Vec3<f32> = aabb_simd.max.into();

      let p0 = Vec3::new(min.x, min.y, min.z);
      let p1 = Vec3::new(min.x, min.y, max.z);
      let p2 = Vec3::new(min.x, max.y, min.z);
      let p3 = Vec3::new(min.x, max.y, max.z);
      let p4 = Vec3::new(max.x, min.y, min.z);
      let p5 = Vec3::new(max.x, min.y, max.z);
      let p6 = Vec3::new(max.x, max.y, min.z);
      let p7 = Vec3::new(max.x, max.y, max.z);

      let lines = &mut depth_lines[depth];
      // bottom face
      lines.push((p0, p2));
      lines.push((p2, p6));
      lines.push((p6, p4));
      lines.push((p4, p0));
      // top face
      lines.push((p1, p3));
      lines.push((p3, p7));
      lines.push((p7, p5));
      lines.push((p5, p1));
      // vertical edges
      lines.push((p0, p1));
      lines.push((p2, p3));
      lines.push((p6, p7));
      lines.push((p4, p5));
    }

    if !node.is_leaf() {
      for lane in 0..QBVH_SIMD_WIDTH {
        let child = node.children[lane];
        if child != u32::MAX && (child as usize) < nodes.len() {
          stack.push((child, depth + 1));
        }
      }
    }
  }

  depth_lines
}

#[derive(Clone)]
pub struct SceneBVHResultView {
  pub bvh: LockReadGuardHolder<SceneQbvh>,
  pub inv: BoxedDynMultiQuery<RawEntityHandle, RawEntityHandle>,
}

impl SceneBVHResultView {
  pub fn iter_unbound_item<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
  ) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + 'a> {
    self
      .inv
      .access_multi(&scene.into_raw())
      .map(|v| {
        Box::new(v.map(|v| unsafe { EntityHandle::from_raw(v) }))
          as Box<dyn Iterator<Item = _> + 'a>
      })
      .unwrap_or_else(|| Box::new([].into_iter()))
  }
}

/// margin is necessary for line-like primitives
///
/// if input bbox is none, it means the sm is unbound and should be considered separately
pub fn use_scene_qbvh(
  cx: &mut impl DBHookCxLike,
  world_bounding: UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = Option<Box3<f64>>>>,
  margin: UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = f32>>,
) -> UseResult<SceneBVHResultView> {
  let (world_bounding, world_bounding_) = world_bounding.fork();

  let unbound_sm_rev_view = world_bounding_
    .dual_query_filter_map(|v| if v.is_none() { Some(()) } else { None })
    .dual_query_intersect(cx.use_dual_query::<SceneModelBelongsToScene>())
    .dual_query_boxed()
    .dual_query_filter_map(|(_, scene_id)| scene_id)
    .dual_query_boxed()
    .use_dual_query_hash_many_to_one(cx)
    .use_assure_result(cx);

  let (cx, bvh) = cx.use_sharable_plain_state(SceneQbvh::default);

  let bvh_ = bvh.clone();
  let ids = cx
    .use_dual_query::<SceneModelBelongsToScene>()
    .dual_query_filter_map(|v| v);

  let compute = world_bounding
    .join(margin)
    .join(ids)
    .map_spawn_stage_in_thread(
      cx,
      |((w, m), sid)| w.has_delta_hint() || m.has_delta_hint() || sid.has_delta_hint(),
      move |((world_bounding, margin), sid)| {
        let mut bvh = bvh_.write();
        let (view, delta) = world_bounding.view_delta();
        let view = view.skip_generation_check::<SceneModelEntity>();

        let (m_view, m_delta) = margin.view_delta();
        let m_view = m_view.skip_generation_check::<SceneModelEntity>();

        let (sid_view, sid_change) = sid.view_delta();
        let sid_view = sid_view.skip_generation_check::<SceneModelEntity>();

        update_qbvh(
          &mut bvh,
          // note: map_changes_key to convert handle to index is ok
          // same index add, remove will be correctly expressed as a change in index.
          delta
            .into_change()
            .collective_filter_map(|v| v)
            .map_changes_key(|k| k.index()),
          sid_change,
          |index| sid_view.access(&index),
          |index| view.access(&index).flatten(),
          m_delta.into_change().map_changes_key(|k| k.index()),
          // note: we here not require strict value scope for margin, the default margin is always 0.
          |index| m_view.access(&index).unwrap_or(0.),
        );
      },
    );

  let _ = compute.use_assure_result(cx);

  if cx.is_resolve_stage() {
    let (inv, _, _) = unbound_sm_rev_view
      .expect_resolve_stage()
      .inv_view_view_delta();
    let inv = inv.into_boxed_multi();
    UseResult::ResolveStageReady(SceneBVHResultView {
      bvh: bvh.make_read_holder(),
      inv,
    })
  } else {
    UseResult::NotInStage
  }
}

fn update_qbvh(
  bvh: &mut SceneQbvh,
  world_bounding_changes: impl DataChanges<Key = u32, Value = Box3<f64>>,
  // smid -> sid
  scene_id_change: impl Query<Key = RawEntityHandle, Value = ValueChange<RawEntityHandle>>,
  scene_id: impl Fn(u32) -> Option<RawEntityHandle>,
  world_bounding_view: impl Fn(u32) -> Option<Box3<f64>>,
  margin_changes: impl DataChanges<Key = u32, Value = f32>,
  margin_view: impl Fn(u32) -> f32,
) {
  for (k, scene_id_change) in scene_id_change.iter_key_value() {
    let k = k.index();
    if let Some(old) = scene_id_change.old_value() {
      bvh.get_or_create_qbvh(*old).remove(k);
    }
    if let Some(new) = scene_id_change.new_value() {
      if let Some(bounding) = world_bounding_view(k) {
        if !bounding.is_empty() {
          bvh
            .get_or_create_qbvh(*new)
            .pre_update_bounding_or_insert(k);
        }
      }
    }
  }

  for k in world_bounding_changes.iter_removed() {
    debug_log!("remove with id: {k}");
    if let Some(scene_id) = scene_id(k) {
      let bvh = bvh.get_or_create_qbvh(scene_id);
      bvh.remove(k);
    } else {
      log::warn!("bounding change unable to access scene id")
    }
  }

  for (k, v) in world_bounding_changes.iter_update_or_insert() {
    if let Some(scene_id) = scene_id(k) {
      let bvh = bvh.get_or_create_qbvh(scene_id);
      if v.is_empty() {
        debug_log!("the bounding of item with id: {k} has been downgraded");
        bvh.remove(k);
      } else {
        debug_log!("pre update with id: {k}, bounding: {v:?}");
        bvh.pre_update_bounding_or_insert(k);
      }
    } else {
      log::warn!("bounding change unable to access scene id")
    }
  }

  bvh.flush_changed_bvh(|bvh| {
    bvh.refit_bounding(|leaf| {
      let bbox = world_bounding_view(*leaf).expect("unable to re get world bounding");
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
  })
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
