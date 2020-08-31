#![allow(dead_code)]
#![allow(unused)]

mod rinecraft;
// mod gui;
mod camera_controls;
mod effect;
mod init;
mod shading;
mod util;
mod vox;
use rendium::application;
use rinecraft::*;

#[tokio::main]
async fn main() {
  env_logger::init();
  application::run::<Rinecraft>("rinecraft");
}

use rendiation_ral::BindGroupHandle;
use rendiation_shadergraph_derives::Shader;
use rendiation_webgpu::*;
use shading::BlockShadingParamGroup;

// new design
// #[derive(Shader)]
struct BlockShader {
  // #[vertex]
  // vertex: Vertex,

  // #[bindgroup]
  parameter: BlockShadingParamGroup,
}

// struct BlockShaderShaderGraphShaderInstance {
//   // vertex: <Vertex as ShaderGraphGeometryProvider>::ShaderGraphGeometryInstance,
//   parameter: <BlockShadingParamGroup as ShaderGraphBindGroupProvider>::ShaderGraphBindGroupInstance,
// }

// impl rendiation_shadergraph::ShaderGraphFactory<WGPURenderer> for BlockShader {
//   type ShaderGraphShaderInstance = BlockShaderShaderGraphShaderInstance;
//   fn create_builder(
//     renderer: &WGPURenderer,
//   ) -> (ShaderGraphBuilder, Self::ShaderGraphShaderInstance) {
//     let builder = ShaderGraphBuilder::new();
//     let instance = BlockShaderShaderGraphShaderInstance {
//       parameter: builder.bindgroup_by::<BlockShadingParamGroup>(renderer),
//     };
//     (builder, instance)
//   }
// }

struct BlockShaderInstance<T: rendiation_ral::RALBackend> {
  parameter: BindGroupHandle<T, BlockShadingParamGroup>,
}

impl rendiation_ral::ShadingProvider<WGPURenderer> for BlockShader {
  fn apply(
    &self,
    render_pass: &mut <WGPURenderer as rendiation_ral::RALBackend>::RenderPass,
    gpu_shading: &<WGPURenderer as rendiation_ral::RALBackend>::Shading,
    resources: &rendiation_ral::BindGroupManager<WGPURenderer>,
  ) {
    render_pass.set_bindgroup(0, resources.get_gpu(self.parameter));
    render_pass.set_bindgroup(1, resources.get_gpu(self.parameter));
  }
}
