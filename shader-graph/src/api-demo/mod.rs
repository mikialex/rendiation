// struct TSSAOShading {
//   sampleCount: usize,
//   VPMatrixInverse: Mat4<f32>,
//   VPMatrix: Mat4<f32>,
//   aoRadius: f32,
// }

// impl ShaderGraphDecorator for TSSAOShading {
//   fn decorate(&mut self, graph: &mut ShaderGraphBuilder) {
//     graph.component(self);

//     let VPMatrix = graph.uniform("VPMatrix");
//     let VPMatrixInverse = graph.uniform("VPMatrixInverse");
//     let sampleCount = graph.uniform("sampleCount");
//     let depthTex = graph.texture("depthResult");
//     let AOAcc = graph.uniform("AOAcc");
//     let aoRadius = graph.uniform("aoRadius");

//     graph.setVertexRoot(screenQuad(graph));

//     let vUV = graph.vary(UvFragVary);
//     let depth = unPackDepth(depthTex.fetch(vUV));

//     let worldPosition = getWorldPosition(vUV, depth, VPMatrix, VPMatrixInverse);
//     let Random2D1 = rand2DT(vUV, sampleCount);
//     let Random2D2 = rand(Random2D1);

//     let randDir = unitDir(Random2D1, Random2D2);

//     let newPositionRand = newSamplePosition(worldPosition.xyz(), aoRadius, randDir, Random2D1);

//     let newDepth = unPackDepth(depthTex.fetch(NDCxyToUV(NDCFromWorldPositionAndVPMatrix(
//       newPositionRand,
//       VPMatrix,
//     ))));

//     graph.set_output0(tssaoMix(
//       AOAcc.fetch(vUV).xyz(),
//       sampleAO(depth, newDepth),
//       sampleCount,
//     ));
//   }
// }

struct VertexProvider {}

#[UniformBuffer]
struct MVPTransformed {
  projection: Mat4<f32>,
  modelView: Mat4<f32>,
}

struct ShaderParameterPool {}

#[derive(BindGroup)]
pub struct BlockShadingParamGroup {
  #[bind_type = "vertex"]
  pub u_mvp_matrix: UniformBuffer,

  #[bind_type = "fragment"]
  pub texture_view: TextureView,

  #[bind_type = "fragment"]
  pub sampler: Sampler,

  #[bind_type = "fragment"]
  pub u_camera_world_position: UniformBuffer,
}

struct BlockShader {
  fog_type: bool,
}

impl_shading!(BlockShader, (BlockShadingParamGroup), (IndexedGeometry));

impl BlockShader {
  pub fn create_shader(&self) -> ShaderGraph {
    let builder = ShaderGraphBuilder::new();
    let geometry = builder.geometry::<IndexedGeometry>();
    let paramter = builder.bindgroup::<BlockShadingParamGroup>();
    let uniform = bindgroup_port.model_view;
    builder.vertex = mvp();

    builder.output()
  }
}

trait ShaderFactory {
  type Builder;
  fn create_shader(&self, builder: Builder) -> ShaderGraph;
}
