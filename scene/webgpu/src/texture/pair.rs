use crate::*;

pub struct GPUTextureSamplerPair {
  pub texture: GPU2DTextureView,
  pub sampler: GPUSamplerView,
}

impl GPUTextureSamplerPair {
  pub fn setup_pass(&self, ctx: &mut GPURenderPassCtx, group: impl Into<usize> + Copy) {
    ctx.binding.bind(&self.texture, group);
    ctx.binding.bind(&self.sampler, group);
  }

  pub fn uniform_and_sample(
    &self,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    group: impl Into<usize> + Copy,
    position: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let texture = binding.uniform_by(&self.texture, group);
    let sampler = binding.uniform_by(&self.sampler, group);
    texture.sample(sampler, position)
  }
}

impl ShareBindableResourceCtx {
  pub fn build_texture_sampler_pair(&self, t: &Texture2DWithSamplingData) -> GPUTextureSamplerPair {
    let sampler = GPUSampler::create(t.sampler.into(), &self.gpu.device);
    let sampler = sampler.create_default_view();

    let ReactiveGPU2DTextureView { gpu: texture, .. } =
      self.get_or_create_reactive_gpu_texture2d(&t.texture);

    GPUTextureSamplerPair { texture, sampler }
  }
}
