mod cube;
pub use cube::*;

mod d2_and_sampler;
pub use d2_and_sampler::*;

use crate::*;

#[derive(Default)]
pub struct TextureGPUSystemSource {
  pub token: UpdateResultToken,
}

impl TextureGPUSystemSource {
  pub fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let default_2d: GPU2DTextureView = create_fallback_empty_texture(&cx.device)
      .create_default_view()
      .try_into()
      .unwrap();
    let texture_2d = gpu_texture_2ds(cx, default_2d.clone());

    let default_sampler = create_gpu_sampler(cx, &TextureSampler::default());
    let samplers = sampler_gpus(cx);

    let bindless_minimal_effective_count = 8192;
    self.token = if is_bindless_supported_on_this_gpu(&cx.info, bindless_minimal_effective_count) {
      let texture_system = BindlessTextureSystemSource::new(
        texture_2d,
        default_2d,
        samplers,
        default_sampler,
        bindless_minimal_effective_count,
      );

      source.register(Box::new(ReactiveQueryBoxAnyResult(texture_system)))
    } else {
      let texture_system = TraditionalPerDrawBindingSystemSource {
        textures: Box::new(texture_2d),
        samplers: Box::new(samplers),
      };
      source.register(Box::new(ReactiveQueryBoxAnyResult(texture_system)))
    };
  }
  pub fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> GPUTextureBindingSystem {
    *res
      .take_result(self.token)
      .unwrap()
      .downcast::<GPUTextureBindingSystem>()
      .unwrap()
  }
}
