use rendiation_shader_library::sampling::random_fn;

use super::*;

/// Samples the shadow map using a PCF kernel made up from random points on a disc
/// aligns with SampleShadowMapRandomDiscPCF in the MJP Shadows sample
#[shader_fn]
pub fn sample_shadow_pcf_random_disc(
  map: BindingNode<ShaderDepthTexture2DArray>,
  d_sampler: BindingNode<ShaderCompareSampler>,
  shadow_position: Node<Vec3<f32>>,
  random_seed: Node<Vec2<f32>>,
  info: Node<ShadowMapAddressInfo>,
  filter_size: Node<Vec2<f32>>,
  num_disc_samples: Node<u32>,
  light_depth_bias: Node<f32>,
  receiver_plane_depth_bias: Node<Vec2<f32>>,
  reversed_depth: Node<bool>,
) -> Node<f32> {
  let map_size = map.texture_dimension_2d(None).into_f32();

  let info_node = info;
  let info = info_node.expand();
  let max_filter_size = val(Vec2::new(MAX_PCF_FILTER_SIZE, MAX_PCF_FILTER_SIZE));
  let filter_size = filter_size.clamp(val(Vec2::new(1., 1.)), max_filter_size);

  let layer = info.layer_index;
  // the depth bias is signed by the caller to move the reference away from the light
  let shadow_depth = shadow_position.z() + light_depth_bias;

  // static depth biasing to make up for incorrect fractional sampling on the shadow map grid
  let fractional_sampling_error =
    fractional_sampling_error_fn(info_node, receiver_plane_depth_bias);
  let shadow_depth =
    apply_fractional_sampling_error_fn(shadow_depth, fractional_sampling_error, reversed_depth);

  // get a value to randomly rotate the kernel by, the rotation is stable
  // across frames only if the caller provides a seed that is stable across frames
  let theta = random_fn(random_seed) * val(6.283_185_3);
  let cos_theta = theta.cos();
  let sin_theta = theta.sin();

  let sample_scale = (val(0.5) * filter_size) / info.size;

  let result = val(0.).make_local_var();

  let i = val(0_u32).make_local_var();
  // the samples are a global const, so the dynamic index in the loop
  // only does a cheap constant array access instead of rebuilding the array
  let samples = global_const_val(POISSON_SAMPLES);
  loop_by(|cx| {
    let idx = i.load();
    if_by(idx.greater_equal_than(num_disc_samples), || cx.do_break());
    let sample_offset = samples.index(idx);
    let sample_offset_rotated = vec2_node((
      sample_offset.x() * cos_theta - sample_offset.y() * sin_theta,
      sample_offset.x() * sin_theta + sample_offset.y() * cos_theta,
    )) * sample_scale;

    let sample_pos = shadow_position.xy() + sample_offset_rotated;

    // compute offset and apply planar depth bias
    let sample_depth = shadow_depth + sample_offset_rotated.dot(receiver_plane_depth_bias);

    let sample = map
      .build_compare_sample_call(
        d_sampler,
        map_uv_to_atlas_uv_fn(sample_pos, info_node, map_size),
        sample_depth,
      )
      .with_array_index(layer)
      .sample();
    result.store(result.load() + sample);

    i.store(idx + val(1));
  });

  result.load() / num_disc_samples.into_f32()
}

/// Poisson disk samples, from the MJP Shadows sample (PCFKernels.hlsl)
pub const POISSON_SAMPLES: [Vec2<f32>; 64] = [
  Vec2::new(-0.511_962_5, -0.482_793_8),
  Vec2::new(-0.217_126_4, -0.476_872_6),
  Vec2::new(-0.755_293_1, -0.242_650_7),
  Vec2::new(-0.713_676_5, -0.449_661_4),
  Vec2::new(-0.593_884_9, -0.689_565_4),
  Vec2::new(-0.314_800_3, -0.704_765_4),
  Vec2::new(-0.422_15, -0.202_460_7),
  Vec2::new(-0.946_681_6, -0.201_450_8),
  Vec2::new(-0.840_906_3, -0.034_657_78),
  Vec2::new(-0.651_757_2, -0.074_763_26),
  Vec2::new(-0.104_182_2, -0.025_212_14),
  Vec2::new(-0.304_271_2, -0.021_954_31),
  Vec2::new(-0.508_230_7, 0.107_980_6),
  Vec2::new(-0.084_298_77, -0.231_629_8),
  Vec2::new(-0.987_912_8, 0.111_368_3),
  Vec2::new(-0.385_963_6, 0.336_354_5),
  Vec2::new(-0.192_533_4, 0.178_728_8),
  Vec2::new(0.003_256_182, 0.138_135),
  Vec2::new(-0.870_683_7, 0.301_067_9),
  Vec2::new(-0.698_203_8, 0.190_432_6),
  Vec2::new(0.197_504_3, 0.222_131_7),
  Vec2::new(0.150_778_8, 0.420_416_8),
  Vec2::new(0.351_405_6, 0.098_655_79),
  Vec2::new(0.155_878_3, -0.084_609_35),
  Vec2::new(-0.068_497_8, 0.446_199_3),
  Vec2::new(0.378_052_2, 0.347_867_9),
  Vec2::new(0.395_679_9, -0.146_917_7),
  Vec2::new(0.583_897_5, 0.105_494_3),
  Vec2::new(0.615_510_5, 0.324_571_6),
  Vec2::new(0.392_862_4, -0.441_762_1),
  Vec2::new(0.174_988_4, -0.420_217_5),
  Vec2::new(0.681_372_7, -0.242_480_8),
  Vec2::new(-0.670_771_1, 0.491_274_1),
  Vec2::new(0.000_513_052_8, -0.805_833_4),
  Vec2::new(0.027_030_13, -0.601_072_8),
  Vec2::new(-0.165_818_8, -0.969_567_4),
  Vec2::new(0.406_059_1, -0.710_072_6),
  Vec2::new(0.771_339_6, -0.471_365_9),
  Vec2::new(0.573_212, -0.515_44),
  Vec2::new(-0.344_889_6, -0.904_649_7),
  Vec2::new(0.126_854_4, -0.987_469_2),
  Vec2::new(0.741_853_3, -0.666_736_6),
  Vec2::new(0.349_252_2, 0.592_466_2),
  Vec2::new(0.567_989_7, 0.534_346_5),
  Vec2::new(0.566_341_7, 0.770_869_8),
  Vec2::new(0.737_549_7, 0.669_141_5),
  Vec2::new(0.227_199_4, -0.616_350_2),
  Vec2::new(0.231_284_4, 0.872_565_9),
  Vec2::new(0.421_699_3, 0.900_283_8),
  Vec2::new(0.426_209_1, -0.901_328_4),
  Vec2::new(0.200_140_8, -0.808_381),
  Vec2::new(0.149_394, 0.665_076_3),
  Vec2::new(-0.096_403_76, 0.984_373_6),
  Vec2::new(0.768_232_8, -0.072_738_44),
  Vec2::new(0.041_465_84, 0.831_318_4),
  Vec2::new(0.970_526_6, -0.114_330_4),
  Vec2::new(0.967_001_7, 0.129_338_5),
  Vec2::new(0.901_503_7, -0.330_694_9),
  Vec2::new(-0.508_564_8, 0.753_417_7),
  Vec2::new(0.905_550_1, 0.375_839_3),
  Vec2::new(0.759_994_6, 0.180_910_9),
  Vec2::new(-0.248_369_5, 0.794_295_2),
  Vec2::new(-0.424_105_2, 0.558_108_7),
  Vec2::new(-0.102_010_6, 0.672_446_8),
];
