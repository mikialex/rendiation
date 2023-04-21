use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct)]
pub struct FlatMaterialUniform {
  pub color: Vec4<f32>,
}

pub struct FlatMaterialGPU {
  uniform: UniformBufferDataView<FlatMaterialUniform>,
}

impl ShaderHashProvider for FlatMaterialGPU {}

impl ShaderGraphProvider for FlatMaterialGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let uniform = binding.uniform_by(&self.uniform, SB::Material).expand();

      builder.register::<DefaultDisplay>(uniform.color);
      Ok(())
    })
  }
}

impl ShaderPassBuilder for FlatMaterialGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.uniform, SB::Material);
  }
}

impl WebGPUMaterial for FlatMaterial {
  type GPU = FlatMaterialGPU;

  fn create_gpu(&self, _: &mut ShareBindableResourceCtx, gpu: &GPU) -> Self::GPU {
    let uniform = FlatMaterialUniform {
      color: self.color,
      ..Zeroable::zeroed()
    };
    let uniform = create_uniform(uniform, gpu);

    FlatMaterialGPU { uniform }
  }

  fn is_keep_mesh_shape(&self) -> bool {
    true
  }

  fn is_transparent(&self) -> bool {
    false
  }
}
