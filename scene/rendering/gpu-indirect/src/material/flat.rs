use crate::*;

pub type FlatMaterialStorageBuffer = ReactiveStorageBufferContainer<FlatMaterialStorage>;

pub fn flat_material_storage_buffer(cx: &GPU) -> FlatMaterialStorageBuffer {
  let color = global_watch().watch::<FlatMaterialDisplayColorComponent>();
  let color_offset = offset_of!(FlatMaterialStorage, color);

  ReactiveStorageBufferContainer::new(cx).with_source(color, color_offset)
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct FlatMaterialStorage {
  pub color: Vec4<f32>,
}

#[derive(Clone)]
pub struct FlatMaterialStorageGPU<'a> {
  pub buffer: &'a FlatMaterialStorageBuffer,
}

impl<'a> ShaderHashProvider for FlatMaterialStorageGPU<'a> {
  shader_hash_type_id! {FlatMaterialStorageGPU<'static>}
}

impl<'a> GraphicsShaderProvider for FlatMaterialStorageGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let materials = binding.bind_by(&self.buffer.inner.gpu());
      let current_material_id = builder.query::<IndirectAbstractMaterialId>();
      let material = materials.index(current_material_id).load().expand();

      builder.register::<DefaultDisplay>(material.color);
    })
  }
}

impl<'a> ShaderPassBuilder for FlatMaterialStorageGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.buffer.inner.gpu());
  }
}
