use crate::*;

pub type FlatMaterialUniforms =
  UniformUpdateContainer<EntityHandle<FlatMaterialEntity>, FlatMaterialUniform>;

pub fn flat_material_uniforms(cx: &GPU) -> FlatMaterialUniforms {
  let color = global_watch()
    .watch::<FlatMaterialDisplayColorComponent>()
    .collective_map(srgb4_to_linear4)
    .into_query_update_uniform(offset_of!(FlatMaterialUniform, color), cx);

  FlatMaterialUniforms::default().with_source(color)
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct FlatMaterialUniform {
  pub color: Vec4<f32>,
}

#[derive(Clone)]
pub struct FlatMaterialGPU<'a> {
  pub uniform: &'a UniformBufferDataView<FlatMaterialUniform>,
}

impl ShaderHashProvider for FlatMaterialGPU<'_> {
  shader_hash_type_id! {FlatMaterialGPU<'static>}
}

impl GraphicsShaderProvider for FlatMaterialGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();

      builder.register::<DefaultDisplay>(uniform.color);
    })
  }
}

impl ShaderPassBuilder for FlatMaterialGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
  }
}
