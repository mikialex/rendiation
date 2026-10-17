use crate::*;

fn model(index: usize) -> EntityHandle<SceneModelEntity> {
  unsafe { EntityHandle::from_raw(RawEntityHandle::create_only_for_testing(index)) }
}

fn hit(t: f64) -> SceneRayHit {
  SceneRayHit {
    model: model(0),
    instance: None,
    primitive: 0,
    t,
    position: Vec3::new(0., 0., -t),
  }
}

fn ray(origin: (f64, f64, f64), direction: (f64, f64, f64)) -> Ray3<f64> {
  Ray3::new(origin.into(), Vec3::from(direction).into_normalized())
}

#[test]
fn ray_aabb_entry() {
  let bbox = Box3::new3(Vec3::splat(-1.), Vec3::splat(1.));

  let outside = ray((0., 0., 5.), (0., 0., -1.));
  assert_eq!(ray_aabb_entry_t(&outside, &bbox), Some(4.));

  // origin inside the box gives zero, not the exit point
  let inside = ray((0., 0., 0.), (0., 0., -1.));
  assert_eq!(ray_aabb_entry_t(&inside, &bbox), Some(0.));

  let miss = ray((3., 0., 5.), (0., 0., -1.));
  assert_eq!(ray_aabb_entry_t(&miss, &bbox), None);

  let backward = ray((0., 0., 5.), (0., 0., 1.));
  assert_eq!(ray_aabb_entry_t(&backward, &bbox), None);

  // degenerate box (a single axis aligned segment) is still hit by a ray passing through it
  let segment = Box3::new3(Vec3::new(-1., 0., 0.), Vec3::new(1., 0., 0.));
  assert_eq!(ray_aabb_entry_t(&outside, &segment), Some(5.));

  assert_eq!(ray_aabb_entry_t(&outside, &Box3::empty()), None);
}

#[test]
fn nearest_with_filter_skips_rejected_hits() {
  let mut v = FilteredRayHits {
    inner: NearestRayHit::default(),
    filter: |h: &SceneRayHit| h.t > 1.5,
  };
  for t in [3., 1., 2.] {
    let _ = v.visit(hit(t));
  }
  assert_eq!(v.inner.nearest.unwrap().t, 2.);
  // the rejected nearer hit must not shrink the bound
  assert_eq!(v.t_max(), 2.);
}

#[test]
fn k_nearest() {
  let mut v = KNearestRayHits::new(2);
  assert_eq!(v.t_max(), f64::INFINITY);
  for t in [5., 1., 3., 2., 4.] {
    let _ = v.visit(hit(t));
  }
  let ts: Vec<_> = v.hits().iter().map(|h| h.t).collect();
  assert_eq!(ts, vec![1., 2.]);
  assert_eq!(v.t_max(), 2.);

  assert_eq!(KNearestRayHits::new(0).t_max(), f64::NEG_INFINITY);
}

#[test]
fn any_hit_breaks() {
  let mut v = AnyRayHit::default();
  assert!(v.visit(hit(1.)).is_break());
  assert!(v.hit.is_some());
}

#[test]
fn local_to_world_t_conversion() {
  let world_mat = Mat4::translate((0., 0., -10.)) * Mat4::scale((2., 2., 2.));
  let world_ray = ray((0., 0., 0.), (0., 0., -1.));
  let local_ray = world_ray
    .apply_matrix_into(world_mat.inverse_or_identity())
    .into_f32();

  let mut inner = AllRayHits::default();
  let mut adapter =
    LocalToWorldRayHitAdapter::new(&mut inner, model(0), world_mat, world_ray, &local_ray);

  // an unbounded inner visitor stays unbounded in local space
  inner_t_max_eq(&adapter, f32::INFINITY);

  // local point z = 1 maps to world z = -8, local t from local origin (z = 5) is 4
  let _ = adapter.visit(LocalRayHit {
    primitive: 3,
    t: 4.,
    position: Vec3::new(0., 0., 1.),
  });
  let world_hit = inner.hits[0];
  assert!((world_hit.t - 8.).abs() < 1e-6);
  assert!((world_hit.position.z + 8.).abs() < 1e-6);
  assert_eq!(world_hit.primitive, 3);

  // a local unit along the ray is 2 world units, so a world bound of 8 is a local bound of 4
  let mut nearest = NearestRayHit::default();
  let _ = nearest.visit(hit(8.));
  let adapter =
    LocalToWorldRayHitAdapter::new(&mut nearest, model(0), world_mat, world_ray, &local_ray);
  inner_t_max_eq(&adapter, 4.);
}

fn inner_t_max_eq(adapter: &LocalToWorldRayHitAdapter, expect: f32) {
  let t_max = adapter.t_max();
  assert!(
    t_max == expect || (t_max - expect).abs() < 1e-5,
    "{t_max} != {expect}"
  );
}

#[test]
fn frustum_world_to_local_matches_plane_transform() {
  let proj = Mat4::perspective_fov_aspect(1., 1.5, 0.1, 100., &OpenGLxNDC);
  let view = Mat4::lookat(Vec3::new(3., 4., 5.), Vec3::zero(), Vec3::new(0., 1., 0.));
  let frustum = Frustum::new_from_matrix(proj * view.inverse_or_identity());

  let world_mat = Mat4::translate((1., -2., 3.)) * Mat4::rotate_y(0.7) * Mat4::scale((2., 0.5, 3.));

  let expect = frustum.apply_matrix_into(world_mat.inverse_or_identity());
  let actual = transform_frustum_world_to_local(&frustum, &world_mat);

  for (e, a) in expect.planes.iter().zip(actual.planes.iter()) {
    assert!((*e.normal - *a.normal).length() < 1e-9);
    assert!((e.constant - a.constant).abs() < 1e-9);
  }
}

#[test]
fn model_frustum_visitors() {
  let mut intersect = ModelFrustumVisitor::new(ObjectTestPolicy::Intersect);
  let _ = intersect.visit(FrustumPrimitiveEvent::single(0, false));
  assert!(!intersect.selected());
  assert!(
    intersect
      .visit(FrustumPrimitiveEvent::single(1, true))
      .is_break()
  );
  assert!(intersect.selected());

  let mut contains = ModelFrustumVisitor::new(ObjectTestPolicy::Contains);
  // an empty model is not contained
  assert!(!contains.selected());
  let _ = contains.visit(FrustumPrimitiveEvent::whole(0, true));
  assert!(!contains.selected());
  let _ = contains.visit(FrustumPrimitiveEvent::whole(3, true));
  assert!(contains.selected());
  assert!(
    contains
      .visit(FrustumPrimitiveEvent::single(4, false))
      .is_break()
  );
  assert!(!contains.selected());
}

#[test]
fn sub_primitive_and_instance_visitors() {
  let mut results = Vec::new();
  let mut v = SubPrimitiveVisitor {
    results: &mut results,
  };
  let _ = v.visit(FrustumPrimitiveEvent::single(0, false));
  let _ = v.visit(FrustumPrimitiveEvent::single(1, true));
  let _ = v.visit(FrustumPrimitiveEvent {
    instance: Some(2),
    primitives: 5..7,
    pass: true,
  });
  let primitives: Vec<_> = results.iter().map(|r| (r.instance, r.primitive)).collect();
  assert_eq!(primitives, vec![(None, 1), (Some(2), 5), (Some(2), 6)]);

  let instance_event = |instance, primitive, pass| FrustumPrimitiveEvent {
    instance: Some(instance),
    primitives: primitive..primitive + 1,
    pass,
  };

  let events = || {
    [
      instance_event(0, 0, true),
      instance_event(0, 1, false),
      instance_event(1, 0, true),
      instance_event(1, 1, true),
      instance_event(2, 0, false),
    ]
  };

  let mut v = InstanceOrSubPrimitiveVisitor::new(ObjectTestPolicy::Intersect);
  events().into_iter().for_each(|e| {
    let _ = v.visit(e);
  });
  let mut results = Vec::new();
  v.finish(&mut results);
  assert_eq!(results, vec![0, 1]);

  let mut v = InstanceOrSubPrimitiveVisitor::new(ObjectTestPolicy::Contains);
  events().into_iter().for_each(|e| {
    let _ = v.visit(e);
  });
  let mut results = Vec::new();
  v.finish(&mut results);
  assert_eq!(results, vec![1]);
}

#[test]
fn model_list_candidates_propagate_break() {
  let models = [model(0), model(1), model(2)];
  let candidates = ModelListCandidates(&models);
  let frustum = SceneFrustumQuery {
    world_frustum: Frustum::default(),
    world_helper: None,
    camera_ctx: CameraQueryCtx {
      camera_view_size_in_logic_pixel: Size::from_u32_pair_min_one((1, 1)),
      pixels_per_unit_calc: Box::new(|_, _| 1.),
      camera_world: Mat4::identity(),
      camera_vp: Mat4::identity(),
      camera_max_scale: 1.,
    },
    extra_screen_space_tolerance: 0.,
  };
  let mut visited = 0;
  let r = candidates.traverse_frustum(model_scene(), &frustum, &mut |_| {
    visited += 1;
    if visited == 2 {
      ControlFlow::Break(())
    } else {
      ControlFlow::Continue(())
    }
  });
  assert!(r.is_break());
  assert_eq!(visited, 2);
}

fn model_scene() -> EntityHandle<SceneEntity> {
  unsafe { EntityHandle::from_raw(RawEntityHandle::create_only_for_testing(0)) }
}
