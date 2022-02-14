use bytemuck::*;
use rendiation_algebra::*;
use rendiation_webgpu::*;

use crate::*;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderUniform)]
pub struct LineMaterial {
  pub color: Vec4<f32>,
}

#[derive(Clone)]
pub struct LineDash {
  pub screen_spaced: bool,
  pub scale: f32,
  pub gap_size: f32,
  pub dash_size: f32,
  pub view_scale: f32,
}

impl SemanticShaderUniform for LineMaterial {
  const TYPE: SemanticBinding = SemanticBinding::Material;
  type Node = Self;
}

pub struct LineMaterialGPU {
  uniform: UniformBuffer<LineMaterial>,
}

impl ShaderGraphProvider for LineMaterialGPU {
  fn build_fragment(
    &self,
    builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let uniform = builder.register_uniform_by(&self.uniform).expand();

    builder.set_fragment_out(0, uniform.color);
    Ok(())
  }
}

impl MaterialCPUResource for LineMaterial {
  type GPU = LineMaterialGPU;

  fn create_gpu(&self, res: &mut GPUResourceSubCache) -> Self::GPU {
    LineMaterialGPU {
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
