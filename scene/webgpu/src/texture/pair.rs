use crate::*;

pub struct GPUTextureSamplerPair {
  pub texture: Texture2DHandle,
  pub sampler: SamplerHandle,
  pub sys: GPUTextureBindingSystem,
}

impl GPUTextureSamplerPair {
  pub fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.sys.bind_texture(&mut ctx.binding, self.texture);
    self.sys.bind_sampler(&mut ctx.binding, self.sampler);
  }

  pub fn bind_and_sample(
    &self,
    binding: &mut ShaderBindGroupDirectBuilder,
    reg: &SemanticRegistry,
    handles: Node<TextureSamplerHandlePair>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let handles = handles.expand();
    self.sys.maybe_sample_texture2d_indirect_and_bind_shader(
      binding,
      reg,
      self.texture,
      handles.texture_handle,
      self.sampler,
      handles.sampler_handle,
      uv,
    )
  }

  pub fn bind_and_sample_enabled(
    &self,
    binding: &mut ShaderBindGroupDirectBuilder,
    reg: &SemanticRegistry,
    handles: Node<TextureSamplerHandlePair>,
    uv: Node<Vec2<f32>>,
  ) -> (Node<Vec4<f32>>, Node<bool>) {
    let handles = handles.expand();
    let r = self.sys.maybe_sample_texture2d_indirect_and_bind_shader(
      binding,
      reg,
      self.texture,
      handles.texture_handle,
      self.sampler,
      handles.sampler_handle,
      uv,
    );
    (r, handles.texture_handle.equals(val(0)))
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Incremental, ShaderStruct, Default, Debug, PartialEq)]
pub struct TextureSamplerHandlePair {
  pub texture_handle: u32,
  pub sampler_handle: u32,
}
