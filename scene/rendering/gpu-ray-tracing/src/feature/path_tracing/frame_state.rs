use super::PTConfig;
use crate::*;

#[derive(Clone)]
pub struct PTRenderState {
  /// the average of the accumulated hdr radiance samples. The average must be done in the
  /// linear space because the monte carlo estimation converges to the expected radiance, and
  /// the tonemap is not linear, averaging the tonemapped samples converges to a different
  /// (darker for the high variance pixels) result.
  pub radiance_buffer: GPU2DTextureView,
  /// the tonemapped radiance buffer for display
  pub display_buffer: GPU2DTextureView,
  sample_count_host: Arc<RwLock<u32>>,
  pub config: UniformBufferDataView<PTConfig>,
}

impl PTRenderState {
  pub fn new(size: Size, max_path_depth: u32, gpu: &GPU) -> Self {
    Self {
      radiance_buffer: create_empty_2d_texture_view(
        gpu,
        size,
        basic_texture_usages(),
        TextureFormat::Rgba32Float,
      ),
      display_buffer: create_empty_2d_texture_view(
        gpu,
        size,
        basic_texture_usages(),
        TextureFormat::Rgba16Float,
      ),
      sample_count_host: Default::default(),
      config: create_uniform(PTConfig::new(max_path_depth), gpu, "PTConfig"),
    }
  }
  pub fn next_sample(&mut self, gpu: &GPU) {
    let current = *self.sample_count_host.read();
    self.config.write_at(&gpu.queue, &(current + 1), 0);
    *self.sample_count_host.write() = current + 1;
  }
  pub fn reset(&mut self, gpu: &GPU) {
    *self.sample_count_host.write() = 0;
    // buffer should be reset automatically in rtx pipeline
    self.config.write_at(&gpu.queue, &0_u32, 0);
  }
}
