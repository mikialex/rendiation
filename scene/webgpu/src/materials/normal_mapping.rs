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
    let st0 = dpdx(vUv.st);
    let st1 = dpdy(vUv.st);

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
  normal_adjust: Node<Vec3<f32>>,
  scale: Node<f32>,
) -> Node<Vec3<f32>> {
  let normal = builder.get_or_compute_fragment_normal();
  let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();
  let position = builder.query_or_interpolate_by::<FragmentWorldPosition, WorldVertexPosition>();
  // let face_dir_sign = builder.query::<FaceSide>

  let normal =
    perturb_normal_2_arb(position, normal, normal_adjust, uv, face_dir_sign) * scale.splat();
  builder.register::<FragmentWorldNormal>(normal);

  normal
}
