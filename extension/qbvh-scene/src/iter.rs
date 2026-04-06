use crate::*;

pub fn create_qbvh_ray_iter<'a>(
  qbvh: &'a SceneQbvh,
  ctx: &'a SceneRayQuery,
  world_ray: &Ray3<f64>,
  tolerance: IntersectTolerance,
) -> impl Iterator<Item = (f32, u32)> + 'a {
  let visitor = RayIntersectionClosestPointVisitor {
    world_ray: world_ray.into_f32(),
    ctx,
    tolerance,
  };

  qbvh.leaf_data_weighted_iter(f32::MAX, visitor)
  // .filter_map(move |(cost, index)| {
  //   models
  //     .get_handle((mapper)(index.into()))
  //     .map(|handle| (cost, handle))
  // });
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
