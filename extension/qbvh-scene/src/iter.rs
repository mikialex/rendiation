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
    let visitor = RayIntersectionClosestPointVisitor {
      world_ray: ctx.world_ray.into_f32(),
      ctx,
      tolerance: IntersectTolerance::new(0., ToleranceType::ScreenSpace),
    };

    let models = global_entity_arena_access::<SceneModelEntity>();
    let iter = self
      .internal
      .leaf_data_weighted_iter(f32::MAX, visitor)
      .map(move |(_cost, index)| {
        let handle = models.get_handle(index as usize).unwrap();
        unsafe { EntityHandle::from_raw(RawEntityHandle::from_handle(handle)) }
      });

    Box::new(iter)
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
      self.ctx.compute_local_tolerance(tolerance, 1.0, center)
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
