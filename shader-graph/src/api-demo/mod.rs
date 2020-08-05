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

#[derive(BindGroup)]
pub struct BlockShadingParamGroup {
  #[bind_stage = "vertex"]
  pub uniforms: MyUniformBuffer,

  #[bind_stage = "fragment"]
  pub texture_view: TextureView,

  #[bind_stage = "fragment"]
  pub sampler: Sampler,

  #[bind_stage = "fragment"]
  pub u_camera_world_position: UniformBuffer,
}

impl BlockShadingParamGroup{
  pub fn create_layout(){

  }

  pub fn create_bindgroup(){

  }
}

struct BlockShadingParamGroupShaderGraphInstance {
  uniforms: MVPTransformedShaderGraphInstance
}

struct BlockShader {
  fog_type: bool,
}

shading_builder!(
  BlockShaderBuilder,
  (block: BlockShadingParamGroup),
  (geometry: IndexedGeometry)
);

struct BlockShaderBuilder {
  builder: ShaderGraphBuilder,
}

impl BlockShaderBuilder {
  // pub fn 
}

impl ShaderFactory for BlockShader {
  type Builder = BlockShaderBuilder;
  fn create_shader(&self, builder: Self::Builder) -> ShaderGraph {
    let builder = ShaderGraphBuilder::new();
    let block_paramter = builder.block();
    let model_view = block_paramter.uniforms.model_view;
    builder.vertex = mvp();

    builder.output()
  }
}

trait ShaderFactory {
  type Builder;
  fn create_shader(&self, builder: Builder) -> ShaderGraph;
}
