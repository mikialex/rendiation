import { WebGLRenderer, SceneGraph } from "../pkg/wasm_scene";
import { WasmSceneGraph } from "./wasm-scene-graph";

export class Renderer{
    constructor(canvas: HTMLCanvasElement) {
        this.wasmRenderer = WebGLRenderer.new(canvas);
    }

    private wasmRenderer: WebGLRenderer

    render(scene: WasmSceneGraph) {
        this.wasmRenderer.render(scene.getWasm());
    }
}