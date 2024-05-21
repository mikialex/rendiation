use crate::*;

// todo, improve performance using self contained collection
pub struct TraditionalPerDrawBindingSystemSource {
  pub textures: Box<dyn ReactiveCollection<Texture2DHandle, GPU2DTextureView>>,
  pub samplers: Box<dyn ReactiveCollection<SamplerHandle, GPUSamplerView>>,
}

impl ReactiveState for TraditionalPerDrawBindingSystemSource {
  type State = TraditionalPerDrawBindingSystem;

  fn poll_current(&mut self, cx: &mut Context) -> Self::State {
    let _ = self.textures.poll_changes(cx);
    let _ = self.samplers.poll_changes(cx);
    TraditionalPerDrawBindingSystem {
      textures: self.textures.access(),
      samplers: self.samplers.access(),
    }
  }
}

pub struct TraditionalPerDrawBindingSystem {
  pub textures: Box<dyn VirtualCollection<Texture2DHandle, GPU2DTextureView>>,
  pub samplers: Box<dyn VirtualCollection<SamplerHandle, GPUSamplerView>>,
}

impl AbstractTraditionalTextureSystem for TraditionalPerDrawBindingSystem {
  fn bind_texture2d(&self, collector: &mut BindingBuilder, handle: Texture2DHandle) {
    let texture = self.textures.access(&handle).unwrap();
    collector.bind(&texture);
  }

  fn bind_sampler(&self, collector: &mut BindingBuilder, handle: SamplerHandle) {
    let sampler = self.samplers.access(&handle).unwrap();
    collector.bind(&sampler);
  }

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> HandleNode<ShaderTexture2D> {
    let texture = self.textures.access(&handle).unwrap();
    builder.bind_by(&texture)
  }

  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> HandleNode<ShaderSampler> {
    let sampler = self.samplers.access(&handle).unwrap();
    builder.bind_by(&sampler)
  }
}
