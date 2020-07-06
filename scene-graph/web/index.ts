// import { intoThree, intoWasmScene } from "./scene";
// import { benchMatrix } from "./bench-matrix-multi";

// // test matrix
// // benchMatrix();

// /// test renderer
// intoThree()
// intoWasmScene();

export const a = 1;

import { SceneShadingDescriptor, SceneShaderDescriptor } from '../pkg/rendiation_scenegraph';

const shader = SceneShaderDescriptor.new("", "");
const shading = SceneShadingDescriptor.new(shader);

console.log(shading) 