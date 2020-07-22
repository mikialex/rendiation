// import { intoThree, intoWasmScene } from "./scene";
// import { benchMatrix } from "./bench-matrix-multi";

// // test matrix
// // benchMatrix();

// /// test renderer
// intoThree()
// intoWasmScene();

import { SceneShadingDescriptor, SceneShaderDescriptor, WebGLRenderer, WASMScene, WebGLBackend } from '../pkg/rendiation_scenegraph';

const shader = new SceneShaderDescriptor("", "");
const shading = new SceneShadingDescriptor(shader);

const canvas = document.getElementById("wasm") as HTMLCanvasElement
const renderer = new WebGLRenderer(canvas);

const backend = new WebGLBackend();

const scene = new WASMScene();
const handle = scene.create_new_node();

console.log(shading) 
console.log(handle) 