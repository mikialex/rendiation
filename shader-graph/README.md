# ShaderGraph

ShaderGraph is a runtime shader linker, the foundation of Rendiation Shading Abstraction Layer(SAL).

SAL aims:

* Write shader effects in component style (Material/Light System)
* Target to many backend API and Platform fallback
* Use different shading language at same time

Every shader module is actually a computation DAG graph, Users can use ShaderGraph API to describe their shading process by construct a DAG graph. In ShaderGraph, every uniform, attribute, any shader input exists as input node. Any function call as compute node, User connect these nodes and give the result node to graph output, maybe vertexPosition or vary in vertex shader, maybe fragColor or MRT output in fragment shader.

To create shader function compute node, user should provide the given function in their preferred shading language(only glsl supported for now) as the node factory. The node factory will allocation compute node in graph and connect them with inputs. To make things go well, we provide several marcos which auto generate this stuff. The marco will create the mapped function in rust with same name in shader, and user can just call them to do graph construction. So the code of graph construct process is very like the common shader.

```rust
// use marco generate a function
glsl_function!("
vec3 uncharted2ToneMapping(
  vec3 intensity,
  float toneMappingExposure,
  float toneMappingWhitePoint
) {
  intensity *= toneMappingExposure;
  return Uncharted2Helper(intensity) / Uncharted2Helper(vec3(toneMappingWhitePoint));
}
");

fn test(){
    // use function to connect node.
    graph.fragColor = uncharted2ToneMapping(intensity, toneMappingExposure, toneMappingWhitePoint)
}
```

This maybe how we design the shading component, WIP

```rust
fn decorate(&mut self, graph: ShaderGraph) {
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

let newDepth = unPackDepth
    (depthTex.fetch(NDCxyToUV(NDCFromWorldPositionAndVPMatrix(newPositionRand, VPMatrix))));

graph.set_output0(tssaoMix(
    AOAcc.fetch(vUV).xyz(),
    sampleAO(depth, newDepth),
    sampleCount,
));
}
```