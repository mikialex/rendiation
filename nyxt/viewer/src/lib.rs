pub use nyxt_core::*;

use rendiation_math::Vec4;
use rendiation_mesh_buffer::vertex::Vertex;
use rendiation_ral::{ShaderGeometryInjected, RAL};
use space_indexer::{
  bvh::BalanceTree,
  bvh::{test::bvh_build, SAH},
  utils::generate_boxes_in_space,
  utils::TreeBuildOption,
};
use wasm_bindgen::prelude::*;

use rendiation_shader_library::transform::*;
use rendiation_shader_library::*;

#[derive(Shader)]
pub struct MeshBasicShader {
  pub uniforms: MeshBasicShaderUniforms,
}

impl<T: RAL> ShaderGeometryInjected<T> for MeshBasicShader {
  type Geometry = Vertex;
}

#[derive(BindGroup)]
pub struct MeshBasicShaderUniforms {
  #[stage(frag)]
  pub parameter: MeshBasicShaderParameter,

  #[stage(vert)]
  pub mvp: CameraTransform,
}

#[derive(UniformBuffer, Copy, Clone)]
#[repr(C, align(16))]
pub struct MeshBasicShaderParameter {
  pub color: Vec4<f32>,
}

#[wasm_bindgen]
pub fn test_bvh() {
  let boxes = generate_boxes_in_space(20000, 10000., 1.);

  for _ in 0..10 {
    let _ = bvh_build(
      &boxes,
      &mut BalanceTree,
      &TreeBuildOption {
        max_tree_depth: 15,
        bin_size: 10,
      },
    );
  }

  let mut sah = SAH::new(4);
  for _ in 0..10 {
    let _ = bvh_build(
      &boxes,
      &mut sah,
      &TreeBuildOption {
        max_tree_depth: 15,
        bin_size: 10,
      },
    );
  }
}

pub use rendiation_shader_library::fog::*;
