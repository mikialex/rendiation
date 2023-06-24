use crate::*;

pub struct GPUTextureSamplerPair {
  pub texture: GPU2DTextureView,
  pub sampler: GPUSamplerView,
}

impl GPUTextureSamplerPair {
  pub fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.texture);
    ctx.binding.bind(&self.sampler);
  }

  pub fn uniform_and_sample(
    &self,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    position: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let texture = binding.uniform_by(&self.texture);
    let sampler = binding.uniform_by(&self.sampler);
    texture.sample(sampler, position)
  }
}

impl ShareBindableResourceCtx {
  pub fn build_reactive_texture_sampler_pair(
    &self,
    t: &Texture2DWithSamplingData,
  ) -> ReactiveGPUTextureSamplerPair {
    let sampler = GPUSampler::create(t.sampler.into_gpu(), &self.gpu.device);
    let sampler = sampler.create_default_view();

    let ReactiveGPU2DTextureView {
      gpu: texture,
      changes,
    } = self.get_or_create_reactive_gpu_texture2d(&t.texture);

    let pair = GPUTextureSamplerPair { texture, sampler };

    ReactiveGPUTextureSamplerPair { pair, changes }
  }
}

#[pin_project::pin_project]
pub struct ReactiveGPUTextureSamplerPair {
  pair: GPUTextureSamplerPair,
  #[pin]
  changes: Texture2dRenderComponentDeltaStream, // todo sampler gpu change streams
}

impl Stream for ReactiveGPUTextureSamplerPair {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.changes.poll_next(cx)
  }
}

impl Deref for ReactiveGPUTextureSamplerPair {
  type Target = GPUTextureSamplerPair;
  fn deref(&self) -> &Self::Target {
    &self.pair
  }
}
