use crate::*;

pub type FlatMaterialUniforms =
  UniformUpdateContainer<EntityHandle<FlatMaterialEntity>, FlatMaterialUniform>;

pub fn flat_material_uniforms(cx: &GPUResourceCtx) -> FlatMaterialUniforms {
  let color = global_watch()
    .watch::<FlatMaterialDisplayColorComponent>()
    .into_uniform_collection_update(offset_of!(FlatMaterialUniform, color), cx);

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

impl<'a> ShaderHashProvider for FlatMaterialGPU<'a> {
  shader_hash_type_id! {FlatMaterialGPU<'static>}
}

impl<'a> GraphicsShaderProvider for FlatMaterialGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, binding| {
      let uniform = binding.bind_by(&self.uniform).load().expand();

      builder.register::<DefaultDisplay>(uniform.color);
      Ok(())
    })
  }
}

impl<'a> ShaderPassBuilder for FlatMaterialGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
  }
}

pub struct FlatMaterialGPUResource {
  uniforms: FlatMaterialUniforms,
}

impl FlatMaterialGPUResource {
  pub fn prepare_render(&self, flat: EntityHandle<FlatMaterialEntity>) -> FlatMaterialGPU {
    FlatMaterialGPU {
      uniform: self.uniforms.get(&flat).unwrap(),
    }
  }
}
