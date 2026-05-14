use database::*;
use dynamic_bvh::*;
use fast_hash_collection::FastHashMap;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_scene_core::*;
use rendiation_scene_geometry_query::*;

pub type SceneDynamicBvhImpl = Bvh;

mod iter;
pub use iter::*;

#[derive(Default)]
pub struct SceneDynamicBvh {
  internal: FastHashMap<RawEntityHandle, (SceneDynamicBvhImpl, bool)>,
}

impl SceneDynamicBvh {
  pub fn get_bvh(&self, scene: RawEntityHandle) -> Option<&SceneDynamicBvhImpl> {
    self.internal.get(&scene).map(|v| &v.0)
  }
  pub fn get_or_create_bvh(&mut self, scene: RawEntityHandle) -> &mut SceneDynamicBvhImpl {
    let bvh = self
      .internal
      .entry(scene)
      .or_insert_with(|| (Default::default(), true));
    bvh.1 = true;
    &mut bvh.0
  }

  pub fn flush_changed_bvh(&mut self, mut f: impl FnMut(&mut SceneDynamicBvhImpl)) {
    for (bvh, changed) in self.internal.values_mut() {
      if *changed {
        *changed = false;
        f(bvh);
      }
    }
  }
}

pub fn generate_dynamic_bvh_wireframe(
  bvh: &SceneDynamicBvhImpl,
) -> Vec<Vec<(Vec3<f32>, Vec3<f32>)>> {
  if bvh.is_empty() {
    return Vec::new();
  }

  let mut depth_lines: Vec<Vec<(Vec3<f32>, Vec3<f32>)>> = Vec::new();
  // stack: (node_id, depth)
  let mut stack: Vec<(u32, usize)> = vec![(0, 0)];

  while let Some((node_id, depth)) = stack.pop() {
    if depth >= depth_lines.len() {
      depth_lines.resize_with(depth + 1, Vec::new);
    }

    let wide = &bvh.nodes()[node_id as usize];
    let left = &wide.left;

    // Emit wireframe for the left child's AABB
    if left.leaf_count() > 0 {
      let aabb = left.aabb();
      let lines = aabb_wireframe_lines(aabb);
      depth_lines[depth].extend(lines);
    }

    // Emit wireframe for the right child's AABB (if present)
    if wide.right.leaf_count() > 0 {
      let aabb = wide.right.aabb();
      let lines = aabb_wireframe_lines(aabb);
      depth_lines[depth].extend(lines);
    }

    // Push children for traversal
    if !left.is_leaf() && left.children as usize > 0 {
      stack.push((left.children, depth + 1));
    }
    if wide.right.leaf_count() > 0 && !wide.right.is_leaf() && wide.right.children as usize > 0 {
      stack.push((wide.right.children, depth + 1));
    }
  }

  depth_lines
}

fn aabb_wireframe_lines(aabb: Box3<f32>) -> [(Vec3<f32>, Vec3<f32>); 12] {
  let min = aabb.min;
  let max = aabb.max;

  let p0 = Vec3::new(min.x, min.y, min.z);
  let p1 = Vec3::new(min.x, min.y, max.z);
  let p2 = Vec3::new(min.x, max.y, min.z);
  let p3 = Vec3::new(min.x, max.y, max.z);
  let p4 = Vec3::new(max.x, min.y, min.z);
  let p5 = Vec3::new(max.x, min.y, max.z);
  let p6 = Vec3::new(max.x, max.y, min.z);
  let p7 = Vec3::new(max.x, max.y, max.z);

  [
    // bottom face
    (p0, p2),
    (p2, p6),
    (p6, p4),
    (p4, p0),
    // top face
    (p1, p3),
    (p3, p7),
    (p7, p5),
    (p5, p1),
    // vertical edges
    (p0, p1),
    (p2, p3),
    (p6, p7),
    (p4, p5),
  ]
}

#[derive(Clone)]
pub struct SceneBVHResultView {
  pub bvh: LockReadGuardHolder<SceneDynamicBvh>,
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
pub fn use_scene_dynamic_bvh(
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

  let (cx, bvh) = cx.use_sharable_plain_state(SceneDynamicBvh::default);

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

        update_dynamic_bvh(
          &mut bvh,
          delta
            .into_change()
            .collective_filter_map(|v| v)
            .map_changes_key(|k| k.index()),
          sid_change,
          |index| sid_view.access(&index),
          |index| view.access(&index).flatten(),
          m_delta.into_change().map_changes_key(|k| k.index()),
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

fn update_dynamic_bvh(
  bvh: &mut SceneDynamicBvh,
  world_bounding_changes: impl DataChanges<Key = u32, Value = Box3<f64>>,
  scene_id_change: impl Query<Key = RawEntityHandle, Value = ValueChange<RawEntityHandle>>,
  scene_id: impl Fn(u32) -> Option<RawEntityHandle>,
  world_bounding_view: impl Fn(u32) -> Option<Box3<f64>>,
  _margin_changes: impl DataChanges<Key = u32, Value = f32>,
  _margin_view: impl Fn(u32) -> f32,
) {
  for (k, scene_id_change) in scene_id_change.iter_key_value() {
    let k = k.index();
    if let Some(old) = scene_id_change.old_value() {
      bvh.get_or_create_bvh(*old).remove(k);
    }
    if let Some(new) = scene_id_change.new_value() {
      if let Some(bounding) = world_bounding_view(k) {
        if !bounding.is_empty() {
          let bbox = Box3::new(bounding.min.into_f32(), bounding.max.into_f32());
          bvh
            .get_or_create_bvh(*new)
            .insert_or_update_partially(bbox, k, 0.0);
        }
      }
    }
  }

  for k in world_bounding_changes.iter_removed() {
    debug_log!("remove with id: {k}");
    if let Some(scene_id) = scene_id(k) {
      let bvh = bvh.get_or_create_bvh(scene_id);
      bvh.remove(k);
    } else {
      log::warn!("bounding change unable to access scene id")
    }
  }

  for (k, v) in world_bounding_changes.iter_update_or_insert() {
    if let Some(scene_id) = scene_id(k) {
      let bvh = bvh.get_or_create_bvh(scene_id);
      if v.is_empty() {
        debug_log!("the bounding of item with id: {k} has been downgraded");
        bvh.remove(k);
      } else {
        debug_log!("pre update with id: {k}, bounding: {v:?}");
        let bbox = Box3::new(v.min.into_f32(), v.max.into_f32());
        bvh.insert_or_update_partially(bbox, k, 0.0);
      }
    } else {
      log::warn!("bounding change unable to access scene id")
    }
  }

  bvh.flush_changed_bvh(|bvh| {
    let mut workspace = BvhWorkspace::default();
    bvh.refit(&mut workspace);
    bvh.optimize_incremental(&mut workspace);

    if ENABLE_SCENE_BVH_DEBUG {
      bvh.assert_well_formed();
    }
  })
}

pub const ENABLE_SCENE_BVH_LOGGING: bool = false;
#[macro_export]
macro_rules! debug_log {
  ($($e:expr),+) => {
    {
      if $crate::ENABLE_SCENE_BVH_LOGGING {
        log::info!($($e),+)
      }
    }
  };
}

pub const ENABLE_SCENE_BVH_DEBUG: bool = false;
