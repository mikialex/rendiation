use crate::*;

pub struct SceneDynamicBvhCandidateProvider {
  pub internal: SceneBVHResultView,
}

impl SceneModelCandidateProvider for SceneDynamicBvhCandidateProvider {
  fn traverse_ray(
    &self,
    scene: EntityHandle<SceneEntity>,
    query: &SceneRayQuery,
    t_max: f64,
    f: &mut dyn FnMut(RayCandidate) -> ControlFlow<(), f64>,
  ) -> ControlFlow<()> {
    let mut t_max = t_max;

    if let Some(bvh) = self.internal.bvh.get_bvh(scene.into_raw())
      && !bvh.is_empty()
    {
      let ray = query.world_ray.into_f32();
      let models = global_entity_arena_access::<SceneModelEntity>();

      // (node, conservative entry t)
      let mut stack = Vec::new();
      push_ray_hit_children(&bvh.nodes()[0], query, &ray, t_max, &mut stack);

      while let Some((node, t_enter)) = stack.pop() {
        if t_enter > t_max {
          continue;
        }
        if node.is_leaf() {
          let model = leaf_to_model(&models, node.children);
          t_max = f(RayCandidate { model, t_enter })?;
        } else {
          let wide = &bvh.nodes()[node.children as usize];
          push_ray_hit_children(wide, query, &ray, t_max, &mut stack);
        }
      }
    }

    for model in self.internal.iter_unbound_item(scene) {
      f(RayCandidate { model, t_enter: 0. })?;
    }

    ControlFlow::Continue(())
  }

  fn traverse_frustum(
    &self,
    scene: EntityHandle<SceneEntity>,
    frustum: &SceneFrustumQuery,
    f: &mut dyn FnMut(FrustumCandidate) -> ControlFlow<()>,
  ) -> ControlFlow<()> {
    if let Some(bvh) = self.internal.bvh.get_bvh(scene.into_raw())
      && !bvh.is_empty()
    {
      let models = global_entity_arena_access::<SceneModelEntity>();
      let mut visit_leaf = |index: u32, known_inside: bool| {
        f(FrustumCandidate {
          model: leaf_to_model(&models, index),
          known_inside,
        })
      };

      let mut stack = vec![0];
      while let Some(node_id) = stack.pop() {
        let wide = &bvh.nodes()[node_id as usize];
        for node in wide.as_array() {
          if node.leaf_count() == 0 {
            continue;
          }
          let aabb = compute_expand_bbox(
            &frustum.camera_ctx,
            node.aabb(),
            node.expansion() + frustum.extra_screen_space_tolerance,
          );

          match f_intersect_exact(
            frustum.world_helper.as_ref(),
            &frustum.world_frustum,
            &aabb.into_f64(),
          ) {
            IntersectResult::Outside => {}
            IntersectResult::Inside => {
              visit_subtree_leaves(bvh, node, &mut |index| visit_leaf(index, true))?;
            }
            IntersectResult::Intersect => {
              if node.is_leaf() {
                visit_leaf(node.children, false)?;
              } else {
                stack.push(node.children);
              }
            }
          }
        }
      }
    }

    for model in self.internal.iter_unbound_item(scene) {
      f(FrustumCandidate {
        model,
        known_inside: false,
      })?;
    }

    ControlFlow::Continue(())
  }
}

/// Test both children of a wide node against the ray, push the hit ones whose entry t is not
/// larger than `t_max`, the farther one first so that the nearer one is visited first.
fn push_ray_hit_children<'a>(
  wide: &'a BvhNodeWide,
  query: &SceneRayQuery,
  ray: &Ray3<f32>,
  t_max: f64,
  stack: &mut Vec<(&'a BvhNode, f64)>,
) {
  let [left, right] = wide.as_array().map(|node| {
    if node.leaf_count() == 0 {
      return None;
    }
    let aabb = compute_expand_bbox(
      &query.camera_ctx,
      node.aabb(),
      node.expansion() + query.extra_screen_space_tolerance,
    );
    let t = ray_aabb_entry_t(ray, &aabb)?;
    // the bvh is tested in f32 while hits are in f64, loosen the entry t a bit to keep it a lower
    // bound of the hit t
    let t = (t as f64 - t.abs() as f64 * 1e-5).max(0.);
    (t <= t_max).then_some((node, t))
  });

  match (left, right) {
    (Some(left), Some(right)) => {
      let (near, far) = if left.1 <= right.1 {
        (left, right)
      } else {
        (right, left)
      };
      stack.push(far);
      stack.push(near);
    }
    (Some(hit), None) | (None, Some(hit)) => stack.push(hit),
    (None, None) => {}
  }
}

fn compute_expand_bbox(cx: &CameraQueryCtx, mut bbox: Box3<f32>, expansion: f32) -> Box3<f32> {
  if expansion <= 0.0 {
    return bbox;
  }

  let tol = IntersectTolerance::new(expansion, ToleranceType::ScreenSpace);
  let tol = cx.compute_local_tolerance(tol, 1., bbox.center().into_f64());

  if tol > 0.0 {
    bbox = bbox.enlarge(tol);
  }

  bbox
}

/// Recursively visit all leaf data under a subtree.
fn visit_subtree_leaves(
  bvh: &Bvh,
  node: &BvhNode,
  f: &mut impl FnMut(u32) -> ControlFlow<()>,
) -> ControlFlow<()> {
  if node.is_leaf() {
    f(node.children)
  } else {
    let wide = &bvh.nodes()[node.children as usize];
    visit_subtree_leaves(bvh, &wide.left, f)?;
    if wide.right.leaf_count() > 0 {
      visit_subtree_leaves(bvh, &wide.right, f)?;
    }
    ControlFlow::Continue(())
  }
}

fn leaf_to_model(models: &TableAllocator, index: u32) -> EntityHandle<SceneModelEntity> {
  let handle = models.get_handle(index as usize).unwrap();
  unsafe { EntityHandle::from_raw(RawEntityHandle::from_handle_impl(handle)) }
}
