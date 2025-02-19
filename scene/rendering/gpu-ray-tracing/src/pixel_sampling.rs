use crate::*;

/// this is useful in offline style ray tracing
pub trait DevicePixelSampleController: Sized {
  /// decide if the current pixel should continue sampling
  fn should_sample(&self) -> Node<bool>;
  /// update the internal control state with new sample result
  fn update_sample_result(&mut self, result: Node<Vec3<f32>>);
  /// get the final result  if the pixel sampling is sufficient
  fn take_result(&self) -> Node<Vec3<f32>>;
  /// return the sample index (how many times this pixel been sampled), the sampler require this
  /// to get correct sample.
  fn next_sample_index(&self) -> Node<u32>;

  /// the generalized sample logic for a pixel, pass the per sample logic in.
  fn sample_pixel(
    mut self,
    mut per_sample: impl FnMut(Node<u32>) -> Node<Vec3<f32>>,
  ) -> Node<Vec3<f32>> {
    loop_by(|cx| {
      if_by(self.should_sample().not(), || {
        cx.do_break();
      });
      self.update_sample_result(per_sample(self.next_sample_index()))
    });
    self.take_result()
  }
}

/// the naive sampler control with fixed sample count for each pixel
pub struct FixedSamplesPerPixelInOneTrace {
  target_samples_per_pixel: u32,
  current_samples: ShaderAccessorOf<u32>,
  accumulate: ShaderAccessorOf<Vec3<f32>>,
}

impl DevicePixelSampleController for FixedSamplesPerPixelInOneTrace {
  fn should_sample(&self) -> Node<bool> {
    self
      .current_samples
      .load()
      .less_than(self.target_samples_per_pixel)
  }

  fn update_sample_result(&mut self, result: Node<Vec3<f32>>) {
    self
      .current_samples
      .store(self.current_samples.load() + val(1));
    self.accumulate.store(self.accumulate.load() + result); // todo should we check overflow?
  }

  fn take_result(&self) -> Node<Vec3<f32>> {
    // todo, assert current samples is not 0
    self.accumulate.load() / self.current_samples.load().into_f32().splat()
  }
  fn next_sample_index(&self) -> Node<u32> {
    self.current_samples.load()
  }
}
