// use crate::*;

// #[derive(Default)]
// pub struct TraditionalPerDrawBindingSystem {
//   textures: Slab<GPU2DTextureView>,
//   samplers: Slab<GPUSamplerView>,
// }

// impl AbstractGPUTextureSystemBase for TraditionalPerDrawBindingSystem {
//   fn register_texture(&mut self, t: GPU2DTextureView, handle: Texture2DHandle) {
//     self.textures.insert(t) as u32
//   }
//   fn deregister_texture(&mut self, t: Texture2DHandle) {
//     self.textures.remove(t as usize);
//   }
//   fn register_sampler(&mut self, t: GPUSamplerView, handle: SamplerHandle) {
//     self.samplers.insert(t) as u32
//   }
//   fn deregister_sampler(&mut self, t: SamplerHandle) {
//     self.samplers.remove(t as usize);
//   }
//   fn maintain(&mut self) {}
// }

// impl AbstractTraditionalTextureSystem for TraditionalPerDrawBindingSystem {
//   fn bind_texture2d(&mut self, collector: &mut BindingBuilder, handle: Texture2DHandle) {
//     let texture = self.textures.get(handle as usize).unwrap();
//     collector.bind(texture);
//   }

//   fn bind_sampler(&mut self, collector: &mut BindingBuilder, handle: SamplerHandle) {
//     let sampler = self.samplers.get(handle as usize).unwrap();
//     collector.bind(sampler);
//   }

//   fn register_shader_texture2d(
//     &self,
//     builder: &mut ShaderBindGroupDirectBuilder,
//     handle: Texture2DHandle,
//   ) -> HandleNode<ShaderTexture2D> {
//     let texture = self.textures.get(handle as usize).unwrap();
//     builder.bind_by(texture)
//   }

//   fn register_shader_sampler(
//     &self,
//     builder: &mut ShaderBindGroupDirectBuilder,
//     handle: SamplerHandle,
//   ) -> HandleNode<ShaderSampler> {
//     let sampler = self.samplers.get(handle as usize).unwrap();
//     builder.bind_by(sampler)
//   }
// }
