use super::*;

/// Samples the shadow map grid-sampled PCF
/// aligns with SampleShadowMapGridPCF in the MJP Shadows sample
#[shader_fn]
pub fn sample_shadow_pcf_grid(
  map: BindingNode<ShaderDepthTexture2DArray>,
  d_sampler: BindingNode<ShaderCompareSampler>,
  shadow_position: Node<Vec3<f32>>,
  info: Node<ShadowMapAddressInfo>,
  filter_size: Node<Vec2<f32>>,
  light_depth_bias: Node<f32>,
  receiver_plane_depth_bias: Node<Vec2<f32>>,
  reversed_depth: Node<bool>,
) -> Node<f32> {
  let info_node = info;
  let info = info_node.expand();
  let max_filter_size = val(Vec2::new(MAX_PCF_FILTER_SIZE, MAX_PCF_FILTER_SIZE));
  let filter_size = filter_size.clamp(val(Vec2::new(1., 1.)), max_filter_size);

  let layer = info.layer_index;
  // the depth bias is signed by the caller to move the reference away from the light
  let shadow_depth = shadow_position.z() + light_depth_bias;

  let map_size = map.texture_dimension_2d(None).into_f32();

  // static depth biasing to make up for incorrect fractional sampling on the shadow map grid
  let fractional_sampling_error =
    fractional_sampling_error_fn(info_node, receiver_plane_depth_bias);
  let shadow_depth =
    apply_fractional_sampling_error_fn(shadow_depth, fractional_sampling_error, reversed_depth);

  let filter_size_greater_than_one = filter_size
    .x()
    .greater_than(val(1.))
    .or(filter_size.y().greater_than(val(1.)));

  filter_size_greater_than_one.select_branched(
    || {
      // get the texel that will be sampled
      let shadow_texel = shadow_position.xy() * info.size;
      let texel_fraction = shadow_texel.fract();

      let radius = filter_size / val(Vec2::new(2., 2.));

      let min_offset = (texel_fraction - radius).floor();
      let max_offset = (texel_fraction + radius).floor();

      let result = val(0.).make_local_var();

      let y = min_offset.y().make_local_var();
      loop_by(|cx| {
        let y_value = y.load();
        let y_weight = y_value.equals(min_offset.y()).select(
          (radius.y() - texel_fraction.y() + val(1.) + y_value).saturate(),
          val(1.),
        );
        let y_weight = y_value.equals(max_offset.y()).select(
          (radius.y() + texel_fraction.y() - y_value).saturate(),
          y_weight,
        );

        let x = min_offset.x().make_local_var();
        loop_by(|cx_inner| {
          let x_value = x.load();
          let x_weight = x_value.equals(min_offset.x()).select(
            (radius.x() - texel_fraction.x() + val(1.) + x_value).saturate(),
            val(1.),
          );
          let x_weight = x_value.equals(max_offset.x()).select(
            (radius.x() + texel_fraction.x() - x_value).saturate(),
            x_weight,
          );

          let sample_offset = vec2_node((x_value, y_value));
          let sample_pos = shadow_position.xy() + sample_offset / info.size;

          // compute offset and apply planar depth bias
          let sample_depth =
            shadow_depth + (sample_offset / info.size).dot(receiver_plane_depth_bias);

          let sample = map
            .build_compare_sample_call(
              d_sampler,
              map_uv_to_atlas_uv_fn(sample_pos, info_node, map_size),
              sample_depth,
            )
            .with_array_index(layer)
            .sample();

          let sample_weight = x_weight * y_weight;
          result.store(result.load() + sample * sample_weight);

          x.store(x_value + val(1.));
          if_by(x_value.greater_equal_than(max_offset.x()), || {
            cx_inner.do_break()
          });
        });

        y.store(y_value + val(1.));
        if_by(y_value.greater_equal_than(max_offset.y()), || cx.do_break());
      });

      result.load() / (filter_size.x() * filter_size.y())
    },
    || {
      // fallback to single sample when the filter size is 1
      map
        .build_compare_sample_call(
          d_sampler,
          map_uv_to_atlas_uv_fn(shadow_position.xy(), info_node, map_size),
          shadow_depth,
        )
        .with_array_index(layer)
        .sample()
    },
  )
}
