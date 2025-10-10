use crate::*;

#[shader_fn]
pub fn perturb_normal_2_arb(
  q0: Node<Vec3<f32>>,
  q1: Node<Vec3<f32>>,
  st0: Node<Vec2<f32>>,
  st1: Node<Vec2<f32>>,
  surf_norm: Node<Vec3<f32>>,
  map_norm: Node<Vec3<f32>>,
  face_dir: Node<f32>,
) -> Node<Vec3<f32>> {
  let n = surf_norm; // normalized

  let q1perp = q1.cross(n);
  let q0perp = n.cross(q0);

  let t = q1perp * st0.x() + q0perp * st1.x();
  let b = q1perp * st0.y() + q0perp * st1.y();

  let det = t.dot(t).max(b.dot(b));

  let scale = det.equals(0.0).select(0.0, face_dir * det.inverse_sqrt());

  (t * (map_norm.x() * scale) + b * (map_norm.y() * scale) + n * map_norm.z()).normalize()
}

pub fn apply_normal_mapping(
  builder: &mut ShaderFragmentBuilderView,
  normal_map_sample: Node<Vec3<f32>>,
  uv: Node<Vec2<f32>>,
  scale: Node<f32>,
) -> Node<Vec3<f32>> {
  let normal = builder.get_or_compute_fragment_normal();
  let position = builder.query_or_interpolate_by::<FragmentRenderPosition, VertexRenderPosition>();

  let normal_adjust = normal_map_sample * val(Vec3::splat(2.)) - val(Vec3::one());
  let normal_adjust = normal_adjust * scale.splat::<Vec3<f32>>();

  let face = builder.query::<FragmentFrontFacing>().select(0., 1.);

  let q0 = position.dpdx();
  let q1 = position.dpdy();
  let st0 = uv.dpdx();
  let st1 = uv.dpdy();

  let normal = perturb_normal_2_arb(q0, q1, st0, st1, normal, normal_adjust, face);
  builder.register::<FragmentRenderNormal>(normal);

  normal
}

pub fn apply_normal_mapping_conditional(
  builder: &mut ShaderFragmentBuilderView,
  normal_map_sample: Node<Vec3<f32>>,
  uv: Node<Vec2<f32>>,
  scale: Node<f32>,
  enabled: Node<bool>,
) -> Node<Vec3<f32>> {
  let normal = builder.get_or_compute_fragment_normal().make_local_var();
  let position = builder.query_or_interpolate_by::<FragmentRenderPosition, VertexRenderPosition>();

  if_by(enabled, || {
    let normal_adjust = normal_map_sample * val(Vec3::splat(2.)) - val(Vec3::one());
    let normal_adjust = normal_adjust * scale.splat::<Vec3<f32>>();

    let face = builder.query::<FragmentFrontFacing>().select(0., 1.);

    let q0 = position.dpdx();
    let q1 = position.dpdy();
    let st0 = uv.dpdx();
    let st1 = uv.dpdy();

    let n = perturb_normal_2_arb(q0, q1, st0, st1, normal.load(), normal_adjust, face);
    normal.store(n);
  });

  let normal = normal.load();
  builder.register::<FragmentRenderNormal>(normal);
  normal
}

pub fn apply_normal_mapping_conditional_uniform_cfg(
  builder: &mut ShaderFragmentBuilderView,
  normal_map_sample: Node<Vec3<f32>>,
  uv: Node<Vec2<f32>>,
  scale: Node<f32>,
  enabled: Node<bool>,
) -> Node<Vec3<f32>> {
  let normal = builder.get_or_compute_fragment_normal().make_local_var();
  let position = builder.query_or_interpolate_by::<FragmentRenderPosition, VertexRenderPosition>();

  let q0 = position.dpdx();
  let q1 = position.dpdy();
  let st0 = uv.dpdx();
  let st1 = uv.dpdy();

  if_by(enabled, || {
    let normal_adjust = normal_map_sample * val(Vec3::splat(2.)) - val(Vec3::one());
    let normal_adjust = normal_adjust * scale.splat::<Vec3<f32>>();

    let face = builder.query::<FragmentFrontFacing>().select(0., 1.);

    let n = perturb_normal_2_arb(q0, q1, st0, st1, normal.load(), normal_adjust, face);
    normal.store(n);
  });

  let normal = normal.load();
  builder.register::<FragmentRenderNormal>(normal);
  normal
}
