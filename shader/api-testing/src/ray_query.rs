use rendiation_shader_api::*;

use crate::harness::*;

/// Use all the fields of the intersection.
fn keep_intersection(intersection: RayIntersection) {
  keep(intersection.kind());
  keep(intersection.t());
  keep(intersection.instance_custom_index());
  keep(intersection.instance_id());
  keep(intersection.sbt_record_offset());
  keep(intersection.geometry_index());
  keep(intersection.primitive_index());
  keep(intersection.barycentrics());
  keep(intersection.front_face().select(val(1), val(0)));
  keep(intersection.object_to_world());
  keep(intersection.world_to_object());
}

/// Trace the ray: initialize the query, proceed in a loop with the candidate intersection
/// access, terminate when the candidate is near enough, and return the committed intersection.
fn trace(
  tlas: BindingNode<ShaderAccelerationStructure>,
  t_max: Node<f32>,
  origin: Node<Vec3<f32>>,
) -> RayIntersection {
  let query = Node::<ShaderRayQuery>::new();
  query.initialize(
    tlas,
    val(0),
    val(0xff),
    val(0.001),
    t_max,
    origin,
    val(Vec3::new(0., 0., 1.)),
  );
  loop_by(|cx| {
    if_by(query.proceed().not(), || cx.do_break());
    let candidate = query.get_candidate_intersection();
    keep_intersection(candidate);
    if_by(candidate.t().less_than(val(1.)), || {
      query.terminate();
      cx.do_break();
    });
  });
  query.get_committed_intersection()
}

/// the ray query in the compute stage, with all the intersection fields
#[test]
fn ray_query_compute() {
  let module = build_compute(|builder| {
    let f = runtime_values(builder).f;
    let tlas = fake_binding(0);
    let committed = trace(tlas, f, f.splat());
    keep_intersection(committed);
    keep(
      committed
        .kind()
        .equals(val(RayIntersectionKind::Triangle as u32))
        .select(committed.t(), f),
    );
  });
  validate(&module);

  let (_, tlas) = module.global_variables.iter().next().unwrap();
  assert_eq!(tlas.space, naga::AddressSpace::Handle);
  assert_eq!(
    tlas.binding,
    Some(naga::ResourceBinding {
      group: 0,
      binding: 0
    })
  );
  assert!(matches!(
    module.types[tlas.ty].inner,
    naga::TypeInner::AccelerationStructure { .. }
  ));
  let queries = module.entry_points[0]
    .function
    .local_variables
    .iter()
    .filter(|(_, v)| matches!(module.types[v.ty].inner, naga::TypeInner::RayQuery { .. }))
    .count();
  assert_eq!(queries, 1);
}

/// several ray queries on several acceleration structures, the query state is independent
#[test]
fn ray_query_multiple() {
  check_compute(|builder| {
    let f = runtime_values(builder).f;
    let first = fake_binding(0);
    let second = fake_binding(1);
    let hit = trace(first, f, f.splat());
    let next = trace(second, hit.t(), hit.object_to_world() * val(Vec4::one()));
    keep(next.t() + hit.t());
  });
}

/// the ray query in the fragment stage
#[test]
fn ray_query_fragment() {
  check_graphics(|builder| {
    builder.fragment(|builder, _| {
      let position = builder.query::<FragmentPosition>();
      let tlas = fake_binding(0);
      let committed = trace(tlas, position.z(), position.xyz());
      builder.define_out_by(channel(TextureFormat::R32Float));
      builder.store_fragment_out(0, committed.t());
    });
  });
}
