// import { intoThree, intoWasmScene } from "./scene";
// import { benchMatrix } from "./bench-matrix-multi";

// // test matrix
// // benchMatrix();

// /// test renderer
// intoThree()
// intoWasmScene();
import './declare'

import { SceneShadingDescriptor, SceneShaderDescriptor, WebGLRenderer, WASMScene, WebGLBackend } from '../pkg/rendiation_scenegraph';
import { Scene } from './src/scene';

const shader = new SceneShaderDescriptor("", "");
const shading = new SceneShadingDescriptor(shader);

const canvas = document.getElementById("wasm") as HTMLCanvasElement
const renderer = new WebGLRenderer(canvas);

const backend = new WebGLBackend();

const scene = new Scene();
const node = scene.createNode();
console.log(node)
console.log(node.transform)

console.log(shading)

material.update((m) => {

});