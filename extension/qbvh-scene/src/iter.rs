// use crate::*;

// pub fn create_qbvh_ray_iter(qbvh: &SceneQbvh) -> impl Iterator<Item = (f32, u32)> {
//   let visitor = RayIntersectionClosestPointVisitor {
//     intersectable: &ctx.world_ray,
//     camera_ctx: ctx.camera_ctx,
//     global_tolerance: global_picking_tolerance(ctx.conf),
//   };

//   qbvh
//     .leaf_data_weighted_iter(f32::MAX, visitor)
//     .filter_map(move |(cost, index)| {
//       models
//         .get_handle((mapper)(index.into()))
//         .map(|handle| (cost, handle))
//     });

//   // todo
//   [].into_iter()
// }

// struct RayIntersectionClosestPointVisitor<'a> {
//   world_ray: &'a Ray3,
//   camera_ctx: &'a SceneCameraInteractiveCtx<'a>,
//   global_tolerance: f32,
// }

// impl<'a> SimdBestFirstVisitDecider<SimdBox3> for RayIntersectionClosestPointVisitor<'a> {
//   fn visit(&mut self, bv: &SimdBox3, margin: &SimdRealValue) -> SimdBestFirstVisitStatus<()> {
//     // calculate aabb local picking margin based on camera ctx.
//     let camera_ctx = &self.camera_ctx;
//     let local_margins = array!(|lane| {
//       local_space_picking_tolerance(
//         margin.extract(lane) + self.global_tolerance,
//         ToleranceType::ScreenSpace,
//         1.,
//         camera_ctx,
//         bv.extract(lane).max_corner(camera_ctx.camera_forward),
//       )
//     });

//     // enlarge origin aabb with local margin.
//     let mut aabbs = *bv;
//     aabbs.loosen(local_margins.into());

//     // do ray-aabb simd intersection.
//     let (hit, toi) = aabbs.intersect_ray(self.world_ray, SimdRealValue::splat(f32::MAX));

//     SimdBestFirstVisitStatus::MaybeContinue {
//       weights: toi,
//       mask: hit,
//       results: [None; QBVH_SIMD_WIDTH],
//     }
//   }
// }
