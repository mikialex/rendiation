struct TSSAOShading {
  sampleCount: usize,
  VPMatrixInverse: Mat4<f32>,
  VPMatrix: Mat4<f32>,
  aoRadius: f32,
}

impl ShaderGraphDecorator for TSSAOShading {
  fn decorate(&mut self, graph: &mut ShaderGraphBuilder) {
    graph.component(self);

    let VPMatrix = graph.uniform("VPMatrix");
    let VPMatrixInverse = graph.uniform("VPMatrixInverse");
    let sampleCount = graph.uniform("sampleCount");
    let depthTex = graph.texture("depthResult");
    let AOAcc = graph.uniform("AOAcc");
    let aoRadius = graph.uniform("aoRadius");

    graph.setVertexRoot(screenQuad(graph));

    let vUV = graph.vary(UvFragVary);
    let depth = unPackDepth(depthTex.fetch(vUV));

    let worldPosition = getWorldPosition(vUV, depth, VPMatrix, VPMatrixInverse);
    let Random2D1 = rand2DT(vUV, sampleCount);
    let Random2D2 = rand(Random2D1);

    let randDir = unitDir(Random2D1, Random2D2);

    let newPositionRand = newSamplePosition(worldPosition.xyz(), aoRadius, randDir, Random2D1);

    let newDepth = unPackDepth(depthTex.fetch(NDCxyToUV(NDCFromWorldPositionAndVPMatrix(
      newPositionRand,
      VPMatrix,
    ))));

    graph.set_output0(tssaoMix(
      AOAcc.fetch(vUV).xyz(),
      sampleAO(depth, newDepth),
      sampleCount,
    ));
  }
}

struct VertexProvider{

}

struct MVPTransformed {
  projection: Mat4<f32>,
  view: Mat4<f32>,
  model: Mat4<f32>,
}

struct MyMaterial {
  vertex: VertexProvider,
  mvp: MVPTransformed,

}

// this is what artgl is

// @ShadingComponent()
// export class TSSAOShading extends BaseEffectShading<TSSAOShading> {

//   @ShadingUniform("u_sampleCount")
//   sampleCount: number = 0;

//   @ShadingUniform("VPMatrixInverse")
//   VPMatrixInverse: Matrix4 = new Matrix4()

//   @ShadingUniform("VPMatrix")
//   VPMatrix: Matrix4 = new Matrix4()

//   @ShadingUniform("u_aoRadius")
//   aoRadius: number = 1

//   decorate(graph: ShaderGraph) {
//     const VPMatrix = this.getPropertyUniform('VPMatrix')
//     const sampleCount = this.getPropertyUniform("sampleCount");
//     const depthTex = texture("depthResult");
//     graph .setVertexRoot(screenQuad(graph))

//     const vUV = graph.getVary(UvFragVary);
//     const depth = unPackDepth.make().input("enc", depthTex.fetch(vUV))

//     const worldPosition = getWorldPosition.make()
//       .input("uv", vUV)
//       .input("depth", depth)
//       .input("VPMatrix", VPMatrix)
//       .input("VPMatrixInverse", this.getPropertyUniform("VPMatrixInverse"))

//     const Random2D1 = rand2DT.make()
//       .input("cood", vUV)
//       .input("t", sampleCount)

//     const Random2D2 = rand.make()
//     .input("n", Random2D1)

//     const randDir = unitDir.make()
//       .input("x", Random2D1)
//       .input("y", Random2D2)

//     const newPositionRand = newSamplePosition.make()
//       .input("positionOld", worldPosition.swizzling("xyz"))
//       .input("distance", this.getPropertyUniform("aoRadius"))
//       .input("dir", randDir)
//       .input("rand", Random2D1)

//     const newDepth = unPackDepth.make()
//       .input("enc",
//         depthTex.fetch(
//           NDCxyToUV.make()
//             .input(
//               "ndc", NDCFromWorldPositionAndVPMatrix.make()
//                 .input(
//                   "position", newPositionRand
//                 ).input(
//                   "matrix", VPMatrix
//                 )
//             )
//         )
//     )

//     graph.setFragmentRoot(
//       tssaoMix.make()
//         .input("oldColor", texture("AOAcc").fetch(vUV).swizzling("xyz"))
//         .input("newColor",
//           sampleAO.make()
//             .input("depth", depth)
//             .input("newDepth", newDepth)
//         )
//         .input("sampleCount", sampleCount)
//     )
//   }
// }

glsl!(
  "
vec3 uncharted2ToneMapping(
  vec3 intensity, 
  float toneMappingExposure,
  float toneMappingWhitePoint
) {
  intensity *= toneMappingExposure;
  return Uncharted2Helper(intensity) / Uncharted2Helper(vec3(toneMappingWhitePoint));
}

"
);

// above marco generate this:

#[allow(non_camel_case_types)]
pub struct uncharted2ToneMappingFunction {
  name: &'static str,
  source: &'static str,
}

fn uncharted2ToneMapping(
  intensity: &ShaderGraphNode<Vec3<f32>>,
  toneMappingExposure: &ShaderGraphNode<f32>,
  toneMappingWhitePoint: &ShaderGraphNode<f32>,
) -> ShaderGraphNode<Vec3<f32>> {
}
