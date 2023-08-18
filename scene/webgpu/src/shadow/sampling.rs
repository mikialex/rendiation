use crate::*;

pub fn sample_shadow(
  shadow_position: Node<Vec3<f32>>,
  map: HandleNode<ShaderDepthTexture2DArray>,
  sampler: HandleNode<ShaderCompareSampler>,
  info: Node<ShadowMapAddressInfo>,
) -> Node<f32> {
  let info = info.expand();
  // sample_shadow_pcf_x4
  // map.sample_compare_index(
  //   sampler,
  //   shadow_position.xy(),
  //   info.layer_index,
  //   shadow_position.z(),
  //   None,
  // );

  // sample_shadow_pcf_x4(shadow_position, map, sampler, info)
  sample_shadow_pcf_x36_by_offset(map, shadow_position, sampler, info)
}

fn sample_shadow_pcf_x36_by_offset(
  map: HandleNode<ShaderDepthTexture2DArray>,
  shadow_position: Node<Vec3<f32>>,
  d_sampler: HandleNode<ShaderCompareSampler>,
  info: ENode<ShadowMapAddressInfo>,
) -> Node<f32> {
  let uv = shadow_position.xy();
  let depth = shadow_position.z();
  let layer = info.layer_index;
  let mut ratio = val(0.0);

  let s = 2_i32; // we should write a for here?

  for i in -1..=1 {
    for j in -1..=1 {
      ratio +=
        map.sample_compare_index_level(d_sampler, uv, layer, depth, Some((s * i, s * j).into()));
    }
  }

  ratio / val(9.)
}
