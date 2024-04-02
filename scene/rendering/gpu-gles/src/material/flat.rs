use crate::*;

pub type FlatMaterialUniforms = UniformUpdateContainer<FlatMaterialEntity, FlatMaterialUniform>;

pub fn flat_material_gpus(cx: GPUResourceCtx) -> FlatMaterialUniforms {
  let source = global_watch()
    .watch_typed_key::<FlatMaterialDisplayColorComponent>()
    .into_uniform_collection_update(offset_of!(FlatMaterialUniform, color), cx);

  FlatMaterialUniforms::default().with_source(source)
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct FlatMaterialUniform {
  pub color: Vec4<f32>,
}

#[derive(Clone)]
pub struct FlatMaterialGPU<'a> {
  uniform: &'a UniformBufferDataView<FlatMaterialUniform>,
}

impl<'a> ShaderHashProvider for FlatMaterialGPU<'a> {}

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
  pub fn prepare_render(&self, flat: AllocIdx<FlatMaterialEntity>) -> FlatMaterialGPU {
    FlatMaterialGPU {
      uniform: self.uniforms.get(&flat).unwrap(),
    }
  }
}
