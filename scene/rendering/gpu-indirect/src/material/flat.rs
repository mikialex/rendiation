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
pub struct FlatMaterialStorageGPU {
  pub buffer: StorageBufferReadOnlyDataView<[FlatMaterialStorage]>,
}

impl ShaderHashProvider for FlatMaterialStorageGPU {
  shader_hash_type_id! {}
}

impl GraphicsShaderProvider for FlatMaterialStorageGPU {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let materials = binding.bind_by(&self.buffer);
      let current_material_id = builder.query::<IndirectAbstractMaterialId>();
      let material = materials.index(current_material_id).load().expand();

      builder.register::<DefaultDisplay>(material.color);
    })
  }
}

impl ShaderPassBuilder for FlatMaterialStorageGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.buffer);
  }
}
