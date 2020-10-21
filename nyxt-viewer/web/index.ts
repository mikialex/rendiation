// import { intoThree, intoWasmScene } from "./scene";
// import { benchMatrix } from "./bench-matrix-multi";

// // test matrix
// // benchMatrix();

// /// test renderer
// intoThree()
// intoWasmScene();
import './declare'

import { WASMAttributeBufferF32, WASMAttributeBufferU16, WASMGeometry } from '../pkg/rendiation_scenegraph';
import { IndexBuffer, Viewer } from './src/scene';


const canvas = document.getElementById("wasm") as HTMLCanvasElement

const viewer = new Viewer(canvas);
const node = viewer.createNode();
console.log(node)
console.log(node.transform)

const index = new WASMAttributeBufferU16(new Uint16Array([1, 0, 0]), 3);
const index_ = new IndexBuffer(viewer, index);

const position = new WASMAttributeBufferF32(new Float32Array([1, 2, 3]), 3);
const normal = new WASMAttributeBufferF32(new Float32Array([1, 0, 0]), 3);

// const geometry = new WASMGeometry(index, position);
