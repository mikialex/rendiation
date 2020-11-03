// import { intoThree, intoWasmScene } from "./scene";
// import { benchMatrix } from "./bench-matrix-multi";

// // test matrix
// // benchMatrix();

// /// test renderer
// intoThree()
// intoWasmScene();

import { AttributeBufferF32WASM, AttributeBufferU16WASM, IndexedVertexGeometryWASM, NyxtViewer, IndexBufferWASM, VertexBufferWASM, test_bvh, SceneNodeWASM, DrawcallWASM, FogDataWASM } from '../pkg/nyxt_viewer';


const canvas = document.getElementById("wasm") as HTMLCanvasElement

const viewer = new NyxtViewer(canvas);
const node = new SceneNodeWASM(viewer);

// node.local_matrix.a1 = 2; // this not work

let new_local = node.local_matrix;
new_local.a1 = 2;
node.local_matrix = new_local;
new_local.free();


console.log(node)
console.log(new_local)

const child_node = new SceneNodeWASM(viewer);
node.add_child(child_node);

// const drawcall = new DrawcallWASM(viewer);
// node.push_drawcall(drawcall);

const index = new AttributeBufferU16WASM(new Uint16Array([1, 0, 0]), 3);
const index_buffer = new IndexBufferWASM(viewer, index);

const position = new AttributeBufferF32WASM(new Float32Array([1, 2, 3]), 3);
const position_buffer = new VertexBufferWASM(viewer, position);
const normal = new AttributeBufferF32WASM(new Float32Array([1, 0, 0]), 3);
const normal_buffer = new VertexBufferWASM(viewer, normal);
const uv = new AttributeBufferF32WASM(new Float32Array([1, 0, 0]), 3);
const uv_buffer = new VertexBufferWASM(viewer, uv);

const geometry = new IndexedVertexGeometryWASM(index_buffer, position_buffer, normal_buffer, uv_buffer);
console.log(geometry)

const fog = new FogDataWASM()
console.log(fog)

// const scene_geometry = new Geometry(viewer, geometry)

// console.log(test_bvh());
// test_bvh()