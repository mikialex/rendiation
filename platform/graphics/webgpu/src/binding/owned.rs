use crate::*;

#[derive(Clone)]
pub enum BindingResourceOwned {
  Buffer(GPUBufferView),
  BufferArray(Vec<GPUBufferView>),
  RawSampler(Rc<Sampler>),
  Sampler(GPUSampler),
  SamplerArray(Vec<GPUSampler>),
  RawTextureView(Rc<TextureView>),
  TextureView(GPUTextureView),
  TextureViewArray(Vec<GPUTextureView>),
}

impl BindingResourceOwned {
  pub fn prepare_ref(&self) -> BindingResourceOwnedRef {
    match self {
      BindingResourceOwned::Buffer(buffer) => {
        BindingResourceOwnedRef::Buffer(buffer.as_buffer_binding())
      }
      BindingResourceOwned::BufferArray(array) => BindingResourceOwnedRef::BufferArray(
        array
          .iter()
          .map(|buffer| buffer.as_buffer_binding())
          .collect(),
      ),
      BindingResourceOwned::RawSampler(sampler) => BindingResourceOwnedRef::Sampler(sampler),
      BindingResourceOwned::Sampler(sampler) => {
        BindingResourceOwnedRef::Sampler(&sampler.resource.0)
      }
      BindingResourceOwned::SamplerArray(samplers) => BindingResourceOwnedRef::SamplerArray(
        samplers.iter().map(|s| s.resource.0.as_ref()).collect(),
      ),
      BindingResourceOwned::RawTextureView(view) => {
        BindingResourceOwnedRef::TextureView(view.as_ref())
      }
      BindingResourceOwned::TextureView(view) => BindingResourceOwnedRef::TextureView(&view.view),
      BindingResourceOwned::TextureViewArray(textures) => {
        BindingResourceOwnedRef::TextureViewArray(textures.iter().map(|s| &s.view).collect())
      }
    }
  }
}

pub enum BindingResourceOwnedRef<'a> {
  Buffer(BufferBinding<'a>),
  BufferArray(Vec<BufferBinding<'a>>),
  Sampler(&'a Sampler),
  SamplerArray(Vec<&'a Sampler>),
  TextureView(&'a TextureView),
  TextureViewArray(Vec<&'a TextureView>),
}

impl<'a> BindingResourceOwnedRef<'a> {
  pub fn as_binding_ref(&'a self) -> gpu::BindingResource<'a> {
    match self {
      BindingResourceOwnedRef::Buffer(buffer) => BindingResource::Buffer(buffer.clone()),
      BindingResourceOwnedRef::BufferArray(buffers) => {
        BindingResource::BufferArray(buffers.as_ref())
      }
      BindingResourceOwnedRef::Sampler(sampler) => BindingResource::Sampler(sampler),
      BindingResourceOwnedRef::SamplerArray(samplers) => {
        BindingResource::SamplerArray(samplers.as_ref())
      }
      BindingResourceOwnedRef::TextureView(texture) => BindingResource::TextureView(texture),
      BindingResourceOwnedRef::TextureViewArray(textures) => {
        BindingResource::TextureViewArray(textures.as_ref())
      }
    }
  }
}
