use std::rc::Rc;

use rendiation_algebra::Vec4;
use rendiation_renderable_mesh::vertex::Vertex;
use rendiation_webgpu::*;

use crate::*;

impl MaterialMeshLayoutRequire for FlatMaterial {
  type VertexInput = Vec<Vertex>;
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderUniform)]
pub struct FlatMaterialUniform {
  pub color: Vec4<f32>,
}

impl ShaderBindingProvider for FlatMaterialGPU {
  fn setup_binding<'a>(&'a self, builder: &mut BindGroupBuilder<'a>) {
    builder.setup_uniform(&self.uniform);
  }
}

impl ShaderGraphProvider for FlatMaterialGPU {
  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let uniform = builder.uniform_by(&self.uniform).expand();

    builder.set_fragment_out(0, uniform.color);
    Ok(())
  }
}

pub struct FlatMaterialGPU {
  uniform: MaterialUniform<FlatMaterialUniform>,
}

impl WebGPUMaterial for FlatMaterial {
  type GPU = FlatMaterialGPU;

  fn create_gpu(&self, res: &mut GPUResourceSubCache) -> Self::GPU {
    FlatMaterialGPU {
      uniform: res.uniforms.get(self.uniform),
    }
  }

  fn is_keep_mesh_shape(&self) -> bool {
    true
  }

  fn is_transparent(&self) -> bool {
    false
  }
}
