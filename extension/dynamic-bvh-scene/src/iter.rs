use crate::*;

pub struct SceneDynamicBvhIterProvider {
  pub internal: SceneBVHResultView,
}

impl SceneModelIterProvider for SceneDynamicBvhIterProvider {
  fn create_ray_scene_model_iter<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    ctx: &'a SceneRayQuery,
  ) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + 'a> {
    let base = self.internal.iter_unbound_item(scene);

    if let Some(bvh) = self.internal.bvh.get_bvh(scene.into_raw()) {
      let ray = ctx.world_ray.into_f32();
      let mut hits: Vec<u32> = Vec::new();

      bvh.traverse(|node| {
        // todo, expand node aabb, ctx.extra screen space
        if node.cast_ray(&ray, f32::MAX) == f32::MAX {
          return TraversalAction::Prune;
        }
        if let Some(leaf_id) = node.leaf_data() {
          hits.push(leaf_id);
        }
        TraversalAction::Continue
      });

      let models = global_entity_arena_access::<SceneModelEntity>();
      let iter = hits.into_iter().map(move |index| {
        let handle = models.get_handle(index as usize).unwrap();
        unsafe { EntityHandle::from_raw(RawEntityHandle::from_handle(handle)) }
      });

      Box::new(iter.chain(base))
    } else {
      base
    }
  }

  fn create_frustum_scene_model_iter<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    frustum: &'a SceneFrustumQuery,
  ) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + 'a> {
    let base = self.internal.iter_unbound_item(scene);

    // todo, expand node aabb, ctx.extra screen space
    if let Some(bvh) = self.internal.bvh.get_bvh(scene.into_raw()) {
      let mut inside_leaves: Vec<u32> = Vec::new();
      let mut partial_leaves: Vec<u32> = Vec::new();

      if !bvh.is_empty() {
        let mut stack: Vec<(u32, bool)> = Vec::new(); // (wide_node_id, prefer_left)
        stack.push((0, true));

        while let Some((node_id, _)) = stack.pop() {
          let wide = &bvh.nodes()[node_id as usize];
          let left = &wide.left;

          if left.leaf_count() > 0 {
            let aabb = left.aabb();
            match f_intersect_exact(
              frustum.world_helper.as_ref(),
              &frustum.world_frustum,
              &aabb.into_f64(),
            ) {
              IntersectResult::Outside => {}
              IntersectResult::Inside => {
                collect_subtree_leaves(bvh, left, &mut inside_leaves);
              }
              IntersectResult::Intersect => {
                if left.is_leaf() {
                  partial_leaves.push(left.children);
                } else {
                  stack.push((left.children, true));
                }
              }
            }
          }

          if wide.right.leaf_count() > 0 {
            let aabb = wide.right.aabb();
            match f_intersect_exact(
              frustum.world_helper.as_ref(),
              &frustum.world_frustum,
              &aabb.into_f64(),
            ) {
              IntersectResult::Outside => {}
              IntersectResult::Inside => {
                collect_subtree_leaves(bvh, &wide.right, &mut inside_leaves);
              }
              IntersectResult::Intersect => {
                if wide.right.is_leaf() {
                  partial_leaves.push(wide.right.children);
                } else {
                  stack.push((wide.right.children, true));
                }
              }
            }
          }
        }
      }

      let models = global_entity_arena_access::<SceneModelEntity>();

      let insider = inside_leaves.into_iter().map(|idx| (true, idx));
      let partial = partial_leaves.into_iter().map(|idx| (false, idx));

      let iter = insider.chain(partial).map(move |(_intersect, idx)| {
        let handle = models.get_handle(idx as usize).unwrap();
        unsafe { EntityHandle::from_raw(RawEntityHandle::from_handle(handle)) }
      });
      Box::new(iter.chain(base))
    } else {
      base
    }
  }
}

/// Recursively collect all leaf data under a subtree.
fn collect_subtree_leaves(bvh: &Bvh, node: &BvhNode, out: &mut Vec<u32>) {
  if node.is_leaf() {
    out.push(node.children);
  } else {
    let wide = &bvh.nodes()[node.children as usize];
    collect_subtree_leaves(bvh, &wide.left, out);
    if wide.right.leaf_count() > 0 {
      collect_subtree_leaves(bvh, &wide.right, out);
    }
  }
}

#[derive(PartialEq, Eq, Debug)]
pub enum IntersectResult {
  Outside,
  Inside,
  /// The AABB intersects the frustum but is not fully inside.
  ///
  /// When `helper` is `None` (degenerate frustum or precise test disabled),
  /// this variant may contain **false positives** — some AABBs reported as
  /// `Intersect` may actually be outside. This is because the fallback is a
  /// conservative p-vertex test that only rejects AABBs entirely behind a
  /// single frustum plane.
  Intersect,
}

pub fn f_intersect_exact<T: Scalar>(
  helper: Option<&FrustumIntersectionTestHelper<T>>,
  f: &Frustum<T>,
  box3: &Box3<T>,
) -> IntersectResult {
  if box3.is_empty() {
    return IntersectResult::Outside;
  }

  // fast Inside check via n-vertex (exact for convex frustum)
  let mut inside = true;
  for p in &f.planes {
    if p.distance_to(&box3.min_corner(*p.normal)) <= T::zero() {
      inside = false;
      break;
    }
  }
  if inside {
    return IntersectResult::Inside;
  }

  if frustum_intersect_aabb(helper, f, box3) {
    IntersectResult::Intersect
  } else {
    IntersectResult::Outside
  }
}
