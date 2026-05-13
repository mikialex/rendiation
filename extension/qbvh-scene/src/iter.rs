use crate::*;

pub struct SceneQbvhIterProvider {
  pub internal: SceneBVHResultView,
}

impl SceneModelIterProvider for SceneQbvhIterProvider {
  fn create_ray_scene_model_iter<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    ctx: &'a SceneRayQuery,
  ) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + 'a> {
    let base = self.internal.iter_unbound_item(scene);

    if let Some(bvh) = self.internal.bvh.get_qbvh(scene.into_raw()) {
      let visitor = RayIntersectionClosestPointVisitor {
        world_ray: ctx.world_ray.into_f32(),
        ctx,
        tolerance: IntersectTolerance::new(0., ToleranceType::ScreenSpace),
      };

      let models = global_entity_arena_access::<SceneModelEntity>();
      let iter = bvh
        .leaf_data_weighted_iter(f32::MAX, visitor)
        .map(move |(_cost, index)| {
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

    if let Some(bvh) = self.internal.bvh.get_qbvh(scene.into_raw()) {
      let mut pick_area_visitor = ScenePickAreaDepthFirstVisitor {
        ctx: frustum,
        global_picking_tolerance: 0.,
        inside_leaf_data_iter: LeafDataIter::new(TraverseAll, bvh),
        intersect_leaf_data_iter: LeafDataIter::new(TraverseAll, bvh),
      };

      // traversal with collision test, pick nodes in visit_decider.leaf_data_iter.node_stack
      bvh.traverse_depth_first(&mut pick_area_visitor);

      let insider = pick_area_visitor
        .inside_leaf_data_iter
        .map(|idx| (true, idx));

      let partial = pick_area_visitor
        .intersect_leaf_data_iter
        .map(|idx| (false, idx));

      let models = global_entity_arena_access::<SceneModelEntity>();

      let iter = insider.chain(partial).map(move |(_intersect, idx)| {
        let handle = models.get_handle(idx as usize).unwrap();
        let handle = unsafe { EntityHandle::from_raw(RawEntityHandle::from_handle(handle)) };
        handle
      });
      Box::new(iter.chain(base))
    } else {
      base
    }
  }
}

struct RayIntersectionClosestPointVisitor<'a> {
  world_ray: Ray3,
  ctx: &'a SceneRayQuery,
  tolerance: IntersectTolerance,
}

impl<'a> SimdBestFirstVisitDecider<SimdBox3> for RayIntersectionClosestPointVisitor<'a> {
  fn visit(&mut self, bv: &SimdBox3, margin: &SimdRealValue) -> SimdBestFirstVisitStatus<()> {
    // calculate aabb local picking margin based on camera ctx.
    // todo, impl simd version
    let local_margins = array!(|lane| {
      let margin = margin.extract(lane);
      let tolerance = IntersectTolerance {
        value: self.tolerance.value + margin,
        ty: self.tolerance.ty,
      };
      let bbox = bv.extract(lane);
      let center = Box3::new(bbox.min.into(), bbox.max.into())
        .center()
        .into_f64();
      self
        .ctx
        .camera_ctx
        .compute_local_tolerance(tolerance, 1.0, center)
    });

    // enlarge origin aabb with local margin.
    let mut aabbs = *bv;
    aabbs.loosen(local_margins.into());

    // do ray-aabb simd intersection.
    let (hit, toi) = aabbs.intersect_ray(&self.world_ray, SimdRealValue::splat(f32::MAX));

    SimdBestFirstVisitStatus::MaybeContinue {
      weights: toi,
      mask: hit,
      results: [None; QBVH_SIMD_WIDTH],
    }
  }
}

pub struct ScenePickAreaDepthFirstVisitor<'a, LeafData, F, B, SimdBV> {
  pub ctx: &'a SceneFrustumQuery,
  pub global_picking_tolerance: f32,
  pub inside_leaf_data_iter: LeafDataIter<'a, LeafData, F, B, SimdBV>,
  pub intersect_leaf_data_iter: LeafDataIter<'a, LeafData, F, B, SimdBV>,
}

impl<'a, LeafData, F, B, SimdBV> SimdVisitor<LeafData, SimdBox3, SimdRealValue>
  for ScenePickAreaDepthFirstVisitor<'a, LeafData, F, B, SimdBV>
{
  fn visit(
    &mut self,
    id: u32,
    bv: &SimdBox3,
    margin: &SimdRealValue,
    data: Option<[Option<&LeafData>; QBVH_SIMD_WIDTH]>,
  ) -> SimdVisitStatus {
    let local_margins = array!(|lane| {
      let bbox = bv.extract(lane);
      let bbox = Box3::new(bbox.min.into(), bbox.max.into());

      let margin = margin.extract(lane) + self.global_picking_tolerance;

      self.ctx.camera_ctx.compute_local_tolerance(
        IntersectTolerance::new(margin, ToleranceType::ScreenSpace),
        1.0,
        bbox.center().into_f64(),
      )
    });

    let mut aabbs = *bv;
    aabbs.loosen(local_margins.into());
    let bounding = aabbs.to_merged_aabb();
    let bounding = Box3::new(bounding.min.into(), bounding.max.into());

    match f_intersect_exact(
      self.ctx.world_helper.as_ref(),
      &self.ctx.world_frustum,
      &bounding.into_f64(),
    ) {
      IntersectResult::Outside => SimdVisitStatus::MaybeContinue(SimdBoolValue::splat(false)),
      IntersectResult::Inside => {
        self.inside_leaf_data_iter.join(id);
        SimdVisitStatus::MaybeContinue(SimdBoolValue::splat(false))
      }
      IntersectResult::Intersect => {
        // a parent node is partial intersect, continue visit its children
        if data.is_none() {
          SimdVisitStatus::MaybeContinue(SimdBoolValue::splat(true))
        }
        // a child node is partial intersect, push it into iter.
        else {
          self.intersect_leaf_data_iter.join(id);
          SimdVisitStatus::MaybeContinue(SimdBoolValue::splat(false))
        }
      }
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
