use crate::*;

#[derive(Clone)]
pub enum BindingResourceOwned {
  Buffer(GPUBufferResourceView),
  BufferArray(Arc<Vec<GPUBufferResourceView>>),
  Sampler(GPUSamplerView),
  SamplerArray(Arc<Vec<GPUSamplerView>>),
  RawTextureView(Arc<gpu::TextureView>, usize), // for example surface texture
  TextureView(GPUTextureView),
  TextureViewArray(Arc<Vec<GPUTextureView>>),
  AccelerationStructure(GPUTlasRaw),
}

impl BindingResourceOwned {
  pub fn prepare_ref(&self) -> BindingResourceOwnedRef<'_> {
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
      BindingResourceOwned::Sampler(sampler) => {
        BindingResourceOwnedRef::Sampler(&sampler.resource.0)
      }
      BindingResourceOwned::SamplerArray(samplers) => BindingResourceOwnedRef::SamplerArray(
        samplers.iter().map(|s| s.resource.0.as_ref()).collect(),
      ),
      BindingResourceOwned::RawTextureView(view, _) => {
        BindingResourceOwnedRef::TextureView(view.as_ref())
      }
      BindingResourceOwned::TextureView(view) => BindingResourceOwnedRef::TextureView(&view.view),
      BindingResourceOwned::TextureViewArray(textures) => {
        BindingResourceOwnedRef::TextureViewArray(textures.iter().map(|s| &s.view).collect())
      }
      BindingResourceOwned::AccelerationStructure(tlas) => {
        BindingResourceOwnedRef::AccelerationStructure(tlas.gpu())
      }
    }
  }
}

pub enum BindingResourceOwnedRef<'a> {
  Buffer(gpu::BufferBinding<'a>),
  BufferArray(Vec<gpu::BufferBinding<'a>>),
  Sampler(&'a gpu::Sampler),
  SamplerArray(Vec<&'a gpu::Sampler>),
  TextureView(&'a gpu::TextureView),
  TextureViewArray(Vec<&'a gpu::TextureView>),
  AccelerationStructure(&'a gpu::Tlas),
}

impl<'a> BindingResourceOwnedRef<'a> {
  pub fn as_binding_ref(&'a self) -> gpu::BindingResource<'a> {
    match self {
      BindingResourceOwnedRef::Buffer(buffer) => gpu::BindingResource::Buffer(buffer.clone()),
      BindingResourceOwnedRef::BufferArray(buffers) => {
        gpu::BindingResource::BufferArray(buffers.as_ref())
      }
      BindingResourceOwnedRef::Sampler(sampler) => gpu::BindingResource::Sampler(sampler),
      BindingResourceOwnedRef::SamplerArray(samplers) => {
        gpu::BindingResource::SamplerArray(samplers.as_ref())
      }
      BindingResourceOwnedRef::TextureView(texture) => gpu::BindingResource::TextureView(texture),
      BindingResourceOwnedRef::TextureViewArray(textures) => {
        gpu::BindingResource::TextureViewArray(textures.as_ref())
      }
      BindingResourceOwnedRef::AccelerationStructure(tlas) => {
        gpu::BindingResource::AccelerationStructure(tlas)
      }
    }
  }
}
