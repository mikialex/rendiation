// import { intoThree, intoWasmScene } from "./scene";
// import { benchMatrix } from "./bench-matrix-multi";

// // test matrix
// // benchMatrix();

// /// test renderer
// intoThree()
// intoWasmScene();
import './declare'

import { AttributeBufferF32WASM, AttributeBufferU16WASM, WASMGeometry, NyxtViewer, IndexBufferWASM, VertexBufferWASM, test_bvh } from '../pkg/nyxt_viewer';
// import { IndexBuffer, VertexBuffer, Viewer, Geometry } from './src/scene';


const canvas = document.getElementById("wasm") as HTMLCanvasElement

const viewer = new NyxtViewer(canvas);
// const node = viewer.create_node();
// console.log(node)
// console.log(node.local_matrix)

const index = new AttributeBufferU16WASM(new Uint16Array([1, 0, 0]), 3);
const index_buffer = new IndexBufferWASM(viewer, index);

const position = new AttributeBufferF32WASM(new Float32Array([1, 2, 3]), 3);
const position_buffer = new VertexBufferWASM(viewer, position);
const normal = new AttributeBufferF32WASM(new Float32Array([1, 0, 0]), 3);
const normal_buffer = new VertexBufferWASM(viewer, normal);

// const geometry = new WASMGeometry(index_buffer, position_buffer, normal_buffer);

// const scene_geometry = new Geometry(viewer, geometry)

// console.log(test_bvh());
test_bvh()