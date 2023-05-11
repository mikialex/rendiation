use crate::*;

#[rustfmt::skip]
wgsl_fn!(
  fn sample_shadow_pcf_x4(
    shadow_position: vec3<f32>,
    map: texture_depth_2d_array,
    d_sampler: sampler_comparison,
    info: ShadowMapAddressInfo,
  ) -> f32 {
    return textureSampleCompareLevel(
      map,
      d_sampler,
      shadow_position.xy,
      info.layer_index,
      shadow_position.z
    );
  }
);

#[rustfmt::skip]
wgsl_fn!(
  fn sample_shadow_pcf_x36_by_offset(
    shadow_position: vec3<f32>,
    map: texture_depth_2d_array,
    d_sampler: sampler_comparison,
    info: ShadowMapAddressInfo,
  ) -> f32 {
    var uv = shadow_position.xy;
    var depth = shadow_position.z;
    var layer = info.layer_index;
    var ratio = 0.0;

    ratio += textureSampleCompareLevel(map, d_sampler, uv, layer, depth, vec2<i32>(2, -2));
    ratio += textureSampleCompareLevel(map, d_sampler, uv, layer, depth, vec2<i32>(2, 0));
    ratio += textureSampleCompareLevel(map, d_sampler, uv, layer, depth, vec2<i32>(2, 2));

    ratio += textureSampleCompareLevel(map, d_sampler, uv, layer, depth, vec2<i32>(0, -2));
    ratio += textureSampleCompareLevel(map, d_sampler, uv, layer, depth, vec2<i32>(0, 0));
    ratio += textureSampleCompareLevel(map, d_sampler, uv, layer, depth, vec2<i32>(0, 2));

    ratio += textureSampleCompareLevel(map, d_sampler, uv, layer, depth, vec2<i32>(-2, -2));
    ratio += textureSampleCompareLevel(map, d_sampler, uv, layer, depth, vec2<i32>(-2, 0));
    ratio += textureSampleCompareLevel(map, d_sampler, uv, layer, depth, vec2<i32>(-2, 2));

    return ratio / 9.;
  }
);
