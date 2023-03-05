use crate::*;

wgsl_fn!(
  // Normal Mapping Without Precomputed Tangents
  // http://www.thetenthplanet.de/archives/1180

  fn perturb_normal_2_arb(
    position: vec3<f32>,
    surf_norm: vec3<f32>,
    mapN: vec3<f32>,
    vUv: vec2<f32>,
    faceDirection: f32,
  ) -> vec3<f32> {
    let q0 = dpdx(position.xyz);
    let q1 = dpdy(position.xyz);
    let st0 = dpdx(vUv.xy);
    let st1 = dpdy(vUv.xy);

    let N = surf_norm; // normalized

    let q1perp = cross(q1, N);
    let q0perp = cross(N, q0);

    let T = q1perp * st0.x + q0perp * st1.x;
    let B = q1perp * st0.y + q0perp * st1.y;

    let det = max(dot(T, T), dot(B, B));
    let scale = select(faceDirection * inverseSqrt(det), 0.0, det == 0.0);

    return normalize(T * (mapN.x * scale) + B * (mapN.y * scale) + N * mapN.z);
  }
);

pub fn apply_normal_mapping(
  builder: &mut ShaderGraphFragmentBuilderView,
  normal_map_sample: Node<Vec3<f32>>,
  uv: Node<Vec2<f32>>,
  scale: Node<f32>,
) -> Node<Vec3<f32>> {
  let normal = builder.get_or_compute_fragment_normal();
  let position = builder.query_or_interpolate_by::<FragmentWorldPosition, WorldVertexPosition>();

  let normal_adjust = normal_map_sample * consts(Vec3::splat(2.)) - consts(Vec3::one());
  let normal_adjust = normal_adjust * scale.splat::<Vec3<f32>>();

  // todo, should we move this to upper?
  let face = builder
    .query::<FragmentFrontFacing>()
    .unwrap() // builtin type
    .select(consts(0.), consts(1.));

  let normal = perturb_normal_2_arb(position, normal, normal_adjust, uv, face);
  builder.register::<FragmentWorldNormal>(normal);

  normal
}

wgsl_fn!(
  fn compute_normal_by_dxdy(position: vec3<f32>) -> vec3<f32> {
    /// note, webgpu canvas is left handed
    return normalize(cross(dpdy(position), dpdx(position)));
  }
);
