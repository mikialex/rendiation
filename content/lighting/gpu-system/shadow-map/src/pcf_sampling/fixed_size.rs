use super::*;

/// the filter weight kernels, from the MJP Shadows sample (PCFKernels.hlsl)
const W3: [[f32; 3]; 3] = [[0.5, 1.0, 0.5], [1.0, 1.0, 1.0], [0.5, 1.0, 0.5]];

const W5: [[f32; 5]; 5] = [
  [0.0, 0.5, 1.0, 0.5, 0.0],
  [0.5, 1.0, 1.0, 1.0, 0.5],
  [1.0, 1.0, 1.0, 1.0, 1.0],
  [0.5, 1.0, 1.0, 1.0, 0.5],
  [0.0, 0.5, 1.0, 0.5, 0.0],
];

const W7: [[f32; 7]; 7] = [
  [0.0, 0.0, 0.5, 1.0, 0.5, 0.0, 0.0],
  [0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0],
  [0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5],
  [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
  [0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5],
  [0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0],
  [0.0, 0.0, 0.5, 1.0, 0.5, 0.0, 0.0],
];

const W9: [[f32; 9]; 9] = [
  [0.0, 0.0, 0.0, 0.5, 1.0, 0.5, 0.0, 0.0, 0.0],
  [0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0],
  [0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0],
  [0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5],
  [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
  [0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5],
  [0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0],
  [0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0],
  [0.0, 0.0, 0.0, 0.5, 1.0, 0.5, 0.0, 0.0, 0.0],
];

/// Samples the shadow map with a fixed-size PCF kernel optimized with GatherCmp.
/// Uses code from "Fast Conventional Shadow Filtering" by Holger Gruen, in GPU Pro.
/// aligns with SampleShadowMapFixedSizePCF in the MJP Shadows sample
pub fn sample_shadow_pcf_fixed_size(
  map: BindingNode<ShaderDepthTexture2DArray>,
  d_sampler: BindingNode<ShaderCompareSampler>,
  shadow_position: Node<Vec3<f32>>,
  info: Node<ShadowMapAddressInfo>,
  light_depth_bias: Node<f32>,
  receiver_plane_depth_bias: Node<Vec2<f32>>,
  fixed_filter_size: FixedFilterSize,
  reversed_depth: Node<bool>,
) -> Node<f32> {
  let info_node = info;
  let info = info_node.expand();
  let layer = info.layer_index;

  let map_size = map.texture_dimension_2d(None).into_f32();

  // the depth bias is signed by the caller to move the reference away from the light
  let light_depth = shadow_position.z() + light_depth_bias;

  // static depth biasing to make up for incorrect fractional sampling on the shadow map grid
  let fractional_sampling_error =
    fractional_sampling_error_fn(info_node, receiver_plane_depth_bias);
  let light_depth =
    apply_fractional_sampling_error_fn(light_depth, fractional_sampling_error, reversed_depth);

  let tc = shadow_position.xy();

  let info_size = info.size;

  match fixed_filter_size {
    FixedFilterSize::Filter3x3 => fixed_size_pcf_kernel::<3>(
      map,
      d_sampler,
      tc,
      layer,
      info_node,
      info_size,
      map_size,
      light_depth,
      receiver_plane_depth_bias,
      W3,
    ),
    FixedFilterSize::Filter5x5 => fixed_size_pcf_kernel::<5>(
      map,
      d_sampler,
      tc,
      layer,
      info_node,
      info_size,
      map_size,
      light_depth,
      receiver_plane_depth_bias,
      W5,
    ),
    FixedFilterSize::Filter7x7 => fixed_size_pcf_kernel::<7>(
      map,
      d_sampler,
      tc,
      layer,
      info_node,
      info_size,
      map_size,
      light_depth,
      receiver_plane_depth_bias,
      W7,
    ),
    FixedFilterSize::Filter9x9 => fixed_size_pcf_kernel::<9>(
      map,
      d_sampler,
      tc,
      layer,
      info_node,
      info_size,
      map_size,
      light_depth,
      receiver_plane_depth_bias,
      W9,
    ),
  }
}

/// the fixed size PCF kernel, uses GatherCmp to fetch 2x2 neighbors with a single call,
/// then interpolates the weights with the fractional part of the sample position.
/// note in the col == -fs2 branch the s.w accumulation reads v1[0].w/z with the
/// row - 1 weights instead of the v0[0] cache like s.z does, this asymmetry is
/// kept verbatim from the MJP reference implementation
fn fixed_size_pcf_kernel<const N: usize>(
  map: BindingNode<ShaderDepthTexture2DArray>,
  d_sampler: BindingNode<ShaderCompareSampler>,
  tc: Node<Vec2<f32>>,
  layer: Node<i32>,
  info_node: Node<ShadowMapAddressInfo>,
  info_size: Node<Vec2<f32>>,
  map_size: Node<Vec2<f32>>,
  light_depth: Node<f32>,
  receiver_plane_depth_bias: Node<Vec2<f32>>,
  w: [[f32; N]; N],
) -> Node<f32> {
  let fs2 = (N / 2) as i32;

  // total weight of the kernel
  let weight_sum = w.iter().flatten().sum::<f32>();

  // get the texel that will be sampled
  let stc = tc * info_size + val(Vec2::new(0.5, 0.5));
  let tcs = stc.floor();
  let fc = stc - tcs;
  let tc = tcs / info_size;

  let s = val(Vec4::<f32>::zero()).make_local_var();

  // the max kernel size is 9, so the row cache is at most 5 entries
  let v1 = zeroed_val::<[Vec4<f32>; 5]>().make_local_var();
  let v0 = zeroed_val::<[Vec2<f32>; 5]>().make_local_var();

  for row in (-fs2..=fs2).step_by(2) {
    let r = (row + fs2) as usize;
    for col in (-fs2..=fs2).step_by(2) {
      let c = (col + fs2) as usize;
      let ci = ((col + fs2) / 2) as usize;

      let mut value = w[r][c];
      if col > -fs2 {
        value += w[r][c - 1];
      }
      if col < fs2 {
        value += w[r][c + 1];
      }
      if row > -fs2 {
        value += w[r - 1][c];
        if col < fs2 {
          value += w[r - 1][c + 1];
        }
        if col > -fs2 {
          value += w[r - 1][c - 1];
        }
      }

      if value != 0. {
        // compute offset and apply planar depth bias
        let sample_offset = vec2_node((val(col as f32), val(row as f32))) / info_size;
        let sample_depth = light_depth + sample_offset.dot(receiver_plane_depth_bias);

        let gathered = map
          .build_compare_sample_call(
            d_sampler,
            map_uv_to_atlas_uv_fn(tc, info_node, map_size),
            sample_depth,
          )
          .with_offset((col, row).into())
          .with_array_index(layer)
          .gather(GatherChannel::X);
        v1.index(val(ci as u32)).store(gathered);
      } else {
        v1.index(val(ci as u32)).store(val(Vec4::zero()));
      }

      let fc_x = fc.x();
      let fc_y = fc.y();

      if col == -fs2 {
        s.store(
          s.load()
            + vec4_node((
              (val(1.) - fc_y)
                * (v1.index(val(0_u32)).load().w() * (val(w[r][c]) - val(w[r][c]) * fc_x)
                  + v1.index(val(0_u32)).load().z()
                    * (fc_x * (val(w[r][c]) - val(w[r][c + 1])) + val(w[r][c + 1]))),
              fc_y
                * (v1.index(val(0_u32)).load().x() * (val(w[r][c]) - val(w[r][c]) * fc_x)
                  + v1.index(val(0_u32)).load().y()
                    * (fc_x * (val(w[r][c]) - val(w[r][c + 1])) + val(w[r][c + 1]))),
              if row > -fs2 {
                (val(1.) - fc_y)
                  * (v0.index(val(0_u32)).load().x() * (val(w[r - 1][c]) - val(w[r - 1][c]) * fc_x)
                    + v0.index(val(0_u32)).load().y()
                      * (fc_x * (val(w[r - 1][c]) - val(w[r - 1][c + 1])) + val(w[r - 1][c + 1])))
              } else {
                val(0.)
              },
              if row > -fs2 {
                fc_y
                  * (v1.index(val(0_u32)).load().w() * (val(w[r - 1][c]) - val(w[r - 1][c]) * fc_x)
                    + v1.index(val(0_u32)).load().z()
                      * (fc_x * (val(w[r - 1][c]) - val(w[r - 1][c + 1])) + val(w[r - 1][c + 1])))
              } else {
                val(0.)
              },
            )),
        );
      } else if col == fs2 {
        s.store(
          s.load()
            + vec4_node((
              (val(1.) - fc_y)
                * (v1.index(val(fs2 as u32)).load().w()
                  * (fc_x * (val(w[r][c - 1]) - val(w[r][c])) + val(w[r][c]))
                  + v1.index(val(fs2 as u32)).load().z() * fc_x * val(w[r][c])),
              fc_y
                * (v1.index(val(fs2 as u32)).load().x()
                  * (fc_x * (val(w[r][c - 1]) - val(w[r][c])) + val(w[r][c]))
                  + v1.index(val(fs2 as u32)).load().y() * fc_x * val(w[r][c])),
              if row > -fs2 {
                (val(1.) - fc_y)
                  * (v0.index(val(fs2 as u32)).load().x()
                    * (fc_x * (val(w[r - 1][c - 1]) - val(w[r - 1][c])) + val(w[r - 1][c]))
                    + v0.index(val(fs2 as u32)).load().y() * fc_x * val(w[r - 1][c]))
              } else {
                val(0.)
              },
              if row > -fs2 {
                fc_y
                  * (v1.index(val(fs2 as u32)).load().w()
                    * (fc_x * (val(w[r - 1][c - 1]) - val(w[r - 1][c])) + val(w[r - 1][c]))
                    + v1.index(val(fs2 as u32)).load().z() * fc_x * val(w[r - 1][c]))
              } else {
                val(0.)
              },
            )),
        );
      } else {
        s.store(
          s.load()
            + vec4_node((
              (val(1.) - fc_y)
                * (v1.index(val(ci as u32)).load().w()
                  * (fc_x * (val(w[r][c - 1]) - val(w[r][c])) + val(w[r][c]))
                  + v1.index(val(ci as u32)).load().z()
                    * (fc_x * (val(w[r][c]) - val(w[r][c + 1])) + val(w[r][c + 1]))),
              fc_y
                * (v1.index(val(ci as u32)).load().x()
                  * (fc_x * (val(w[r][c - 1]) - val(w[r][c])) + val(w[r][c]))
                  + v1.index(val(ci as u32)).load().y()
                    * (fc_x * (val(w[r][c]) - val(w[r][c + 1])) + val(w[r][c + 1]))),
              if row > -fs2 {
                (val(1.) - fc_y)
                  * (v0.index(val(ci as u32)).load().x()
                    * (fc_x * (val(w[r - 1][c - 1]) - val(w[r - 1][c])) + val(w[r - 1][c]))
                    + v0.index(val(ci as u32)).load().y()
                      * (fc_x * (val(w[r - 1][c]) - val(w[r - 1][c + 1])) + val(w[r - 1][c + 1])))
              } else {
                val(0.)
              },
              if row > -fs2 {
                fc_y
                  * (v1.index(val(ci as u32)).load().w()
                    * (fc_x * (val(w[r - 1][c - 1]) - val(w[r - 1][c])) + val(w[r - 1][c]))
                    + v1.index(val(ci as u32)).load().z()
                      * (fc_x * (val(w[r - 1][c]) - val(w[r - 1][c + 1])) + val(w[r - 1][c + 1])))
              } else {
                val(0.)
              },
            )),
        );
      }

      if row != fs2 {
        v0.index(val(ci as u32))
          .store(v1.index(val(ci as u32)).load().xy());
      }
    }
  }

  s.load().dot(val(Vec4::one())) / val(weight_sum)
}
