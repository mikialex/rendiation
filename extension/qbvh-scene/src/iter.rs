use crate::*;

pub struct SceneQbvhIterProvider {
  internal: LockReadGuardHolder<SceneQbvh>,
}

impl SceneModelIterProvider for SceneQbvhIterProvider {
  fn create_ray_scene_model_iter<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    ctx: &'a SceneRayQuery,
  ) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + 'a> {
    if let Some(bvh) = self.internal.get_qbvh(scene.into_raw()) {
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

      Box::new(iter)
    } else {
      Box::new([].into_iter())
    }
  }

  fn create_frustum_scene_model_iter<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    frustum: &'a SceneFrustumQuery,
  ) -> Box<dyn Iterator<Item = EntityHandle<SceneModelEntity>> + 'a> {
    if let Some(bvh) = self.internal.get_qbvh(scene.into_raw()) {
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
      Box::new(iter)
    } else {
      Box::new([].into_iter())
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

    match f_intersect(&self.ctx.world_frustum, &bounding.into_f64()) {
      IntersectResult::Outside => SimdVisitStatus::MaybeContinue(SimdBoolValue::splat(false)),
      IntersectResult::Inside => {
        self.inside_leaf_data_iter.join(id);
        SimdVisitStatus::MaybeContinue(SimdBoolValue::splat(false))
      }
      IntersectResult::MaybeIntersect => {
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
  /// todo, this contains lot's of false positive, we should do more culling(following)
  MaybeIntersect,
}

// pub fn intersect<T: Scalar>(f: &Frustum<T>, box3: &HyperAABB<Vec3<T>>) -> IntersectResult {
//   // check Intersect false positive: box intersect frustum,but outside
//   let check_false_positive = || {
//     // box3 is AABB
//     self.bounding.min.x > box3.max.x
//       || box3.min.x > self.bounding.max.x
//       || self.bounding.min.y > box3.max.y
//       || box3.min.y > self.bounding.max.y
//       || self.bounding.min.z > box3.max.z
//       || box3.min.z > self.bounding.max.z
//   };

//   match f_intersect(f, box3) {
//     IntersectResult::Inside => IntersectResult::Inside,
//     IntersectResult::Outside => IntersectResult::Outside,
//     IntersectResult::Intersect => {
//       if check_false_positive() {
//         IntersectResult::Outside
//       } else {
//         IntersectResult::Intersect
//       }
//     }
//   }
// }

pub fn f_intersect<T: Scalar>(f: &Frustum<T>, box3: &HyperAABB<Vec3<T>>) -> IntersectResult {
  if box3.is_empty() {
    return IntersectResult::Outside;
  }

  let mut num: u32 = 0;
  for p in f.planes {
    let point_p = box3.min_corner(*p.normal);
    let point_q = box3.max_corner(*p.normal);
    if p.distance_to(&point_q) < T::zero() {
      return IntersectResult::Outside;
    }
    if p.distance_to(&point_p) > T::zero() {
      num += 1;
    }
  }
  if num == 6 {
    return IntersectResult::Inside;
  }
  IntersectResult::MaybeIntersect
}
