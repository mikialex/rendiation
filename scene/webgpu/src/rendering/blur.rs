use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct LinearBlurConfig {
  pub direction: Vec2<f32>,
}

pub struct ShaderSamplingWeights {
  /// we max support 32 weight, but maybe not used them all.
  /// this array is just used as a fixed size container.
  weights: [f32; 32],
  /// the actually sample count we used.
  weight_count: u32,
}

pub struct LinearBlurTask<'a, T> {
  input: AttachmentReadView<T>,
  lighter: &'a LinearBlurConfig,
}

wgsl_function!(
  fn lin_space(w0: f32, d0: vec4<f32>, w1: f32, d1: vec4<f32>) -> f32 {
    return (w0 * d0 + w1 * d1);
  }
);

// wgsl_function!(
//   fn linear_blur(
//     direction: vec2<f32>,
//     weights: ShaderSamplingWeights,
//     texture: texture_2d<f32>,
//     sp: sampler,
//     uv: vec2<f32>,
//     texel_size: vec2<f32>
//   ) -> f32 {
//     let sample_offset = texel_size * direction;
//     var sum: vec4<f32>;
//     for (var i: i32 = 2; i < weights.weight_count; i++) {
//         vec4 samples = textureSample(texture, sp, uv + float(i) * sample_offset);
//         sum = lin_space(1.0, sum, weights.weights[i], samples);
//     }
//   }
// );
