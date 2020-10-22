// import { intoThree, intoWasmScene } from "./scene";
// import { benchMatrix } from "./bench-matrix-multi";

// // test matrix
// // benchMatrix();

// /// test renderer
// intoThree()
// intoWasmScene();
import './declare'

import { WASMAttributeBufferF32, WASMAttributeBufferU16, WASMGeometry } from '../pkg/nyxt_viewer';
import { IndexBuffer, VertexBuffer, Viewer, Geometry } from './src/scene';


const canvas = document.getElementById("wasm") as HTMLCanvasElement

const viewer = new Viewer(canvas);
const node = viewer.createNode();
console.log(node)
console.log(node.transform)

const index = new WASMAttributeBufferU16(new Uint16Array([1, 0, 0]), 3);
const index_buffer = new IndexBuffer(viewer, index);

const position = new WASMAttributeBufferF32(new Float32Array([1, 2, 3]), 3);
const position_buffer = new VertexBuffer(viewer, position);
const normal = new WASMAttributeBufferF32(new Float32Array([1, 0, 0]), 3);
const normal_buffer = new VertexBuffer(viewer, normal);

const geometry = new WASMGeometry(index_buffer.handle, position_buffer.handle, normal_buffer.handle);

const scene_geometry = new Geometry(viewer, geometry)
