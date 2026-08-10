use super::*;

/// Samples the shadow map with a fixed-size PCF kernel optimized with bilinear weighted samples
/// uses the method from The Witness
/// aligns with SampleShadowMapOptimizedPCF in the MJP Shadows sample
#[shader_fn]
pub fn sample_shadow_pcf_optimized(
  map: BindingNode<ShaderDepthTexture2DArray>,
  d_sampler: BindingNode<ShaderCompareSampler>,
  shadow_position: Node<Vec3<f32>>,
  info: Node<ShadowMapAddressInfo>,
  receiver_plane_depth_bias: Node<Vec2<f32>>,
  reversed_depth: Node<bool>,
) -> Node<f32> {
  let info_node = info;
  let info = info_node.expand();
  let layer = info.layer_index;

  let map_size = map.texture_dimension_2d(None).into_f32();

  let light_depth = shadow_position.z();

  // static depth biasing to make up for incorrect fractional sampling on the shadow map grid
  // the optimized PCF samples at offsets up to 1.5 texels, so the error is doubled
  // compared to the other PCF modes
  let fractional_sampling_error =
    fractional_sampling_error_fn(info_node, receiver_plane_depth_bias) * val(2.);
  let light_depth =
    apply_fractional_sampling_error_fn(light_depth, fractional_sampling_error, reversed_depth);

  // 1 unit = 1 texel
  let uv = shadow_position.xy() * info.size;

  let half = val(Vec2::new(0.5, 0.5));
  let base_uv = (uv + half).floor();
  let s = uv.x() + val(0.5) - base_uv.x();
  let t = uv.y() + val(0.5) - base_uv.y();

  let base_uv = (base_uv - half) / info.size;

  // sample the shadow map at the given offset, with the receiver plane depth bias applied
  let sample = |u: Node<f32>, v: Node<f32>| {
    let offset = vec2_node((u, v)) / info.size;
    let sample_pos = base_uv + offset;
    let sample_depth = light_depth + offset.dot(receiver_plane_depth_bias);
    map
      .build_compare_sample_call(
        d_sampler,
        map_uv_to_atlas_uv_fn(sample_pos, info_node, map_size),
        sample_depth,
      )
      .with_array_index(layer)
      .sample()
  };

  if OPTIMIZED_PCF_FILTER_SIZE == 3 {
    let uw0 = val(3.) - val(2.) * s;
    let uw1 = val(1.) + val(2.) * s;
    let u0 = (val(2.) - s) / uw0 - val(1.);
    let u1 = s / uw1 + val(1.);

    let vw0 = val(3.) - val(2.) * t;
    let vw1 = val(1.) + val(2.) * t;
    let v0 = (val(2.) - t) / vw0 - val(1.);
    let v1 = t / vw1 + val(1.);

    let mut sum = val(0.);
    sum += uw0 * vw0 * sample(u0, v0);
    sum += uw1 * vw0 * sample(u1, v0);
    sum += uw0 * vw1 * sample(u0, v1);
    sum += uw1 * vw1 * sample(u1, v1);
    sum / val(16.)
  } else if OPTIMIZED_PCF_FILTER_SIZE == 5 {
    let uw0 = val(4.) - val(3.) * s;
    let uw1 = val(7.);
    let uw2 = val(1.) + val(3.) * s;

    let u0 = (val(3.) - val(2.) * s) / uw0 - val(2.);
    let u1 = (val(3.) + s) / uw1;
    let u2 = s / uw2 + val(2.);

    let vw0 = val(4.) - val(3.) * t;
    let vw1 = val(7.);
    let vw2 = val(1.) + val(3.) * t;

    let v0 = (val(3.) - val(2.) * t) / vw0 - val(2.);
    let v1 = (val(3.) + t) / vw1;
    let v2 = t / vw2 + val(2.);

    let mut sum = val(0.);
    sum += uw0 * vw0 * sample(u0, v0);
    sum += uw1 * vw0 * sample(u1, v0);
    sum += uw2 * vw0 * sample(u2, v0);
    sum += uw0 * vw1 * sample(u0, v1);
    sum += uw1 * vw1 * sample(u1, v1);
    sum += uw2 * vw1 * sample(u2, v1);
    sum += uw0 * vw2 * sample(u0, v2);
    sum += uw1 * vw2 * sample(u1, v2);
    sum += uw2 * vw2 * sample(u2, v2);
    sum / val(144.)
  } else {
    // OPTIMIZED_PCF_FILTER_SIZE == 7
    let uw0 = val(5.) * s - val(6.);
    let uw1 = val(11.) * s - val(28.);
    let uw2 = -(val(11.) * s + val(17.));
    let uw3 = -(val(5.) * s + val(1.));

    let u0 = (val(4.) * s - val(5.)) / uw0 - val(3.);
    let u1 = (val(4.) * s - val(16.)) / uw1 - val(1.);
    let u2 = -(val(7.) * s + val(5.)) / uw2 + val(1.);
    let u3 = -s / uw3 + val(3.);

    let vw0 = val(5.) * t - val(6.);
    let vw1 = val(11.) * t - val(28.);
    let vw2 = -(val(11.) * t + val(17.));
    let vw3 = -(val(5.) * t + val(1.));

    let v0 = (val(4.) * t - val(5.)) / vw0 - val(3.);
    let v1 = (val(4.) * t - val(16.)) / vw1 - val(1.);
    let v2 = -(val(7.) * t + val(5.)) / vw2 + val(1.);
    let v3 = -t / vw3 + val(3.);

    let mut sum = val(0.);
    sum += uw0 * vw0 * sample(u0, v0);
    sum += uw1 * vw0 * sample(u1, v0);
    sum += uw2 * vw0 * sample(u2, v0);
    sum += uw3 * vw0 * sample(u3, v0);
    sum += uw0 * vw1 * sample(u0, v1);
    sum += uw1 * vw1 * sample(u1, v1);
    sum += uw2 * vw1 * sample(u2, v1);
    sum += uw3 * vw1 * sample(u3, v1);
    sum += uw0 * vw2 * sample(u0, v2);
    sum += uw1 * vw2 * sample(u1, v2);
    sum += uw2 * vw2 * sample(u2, v2);
    sum += uw3 * vw2 * sample(u3, v2);
    sum += uw0 * vw3 * sample(u0, v3);
    sum += uw1 * vw3 * sample(u1, v3);
    sum += uw2 * vw3 * sample(u2, v3);
    sum += uw3 * vw3 * sample(u3, v3);
    sum / val(2704.)
  }
}
