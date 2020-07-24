import { WASMScene } from "../../pkg/rendiation_scenegraph";
import * as wasm from "../../pkg/rendiation_scenegraph_bg.wasm";


let F32WasmMemoryViewCache = null;
function getF32Memory0(): Float32Array {
    if (F32WasmMemoryViewCache === null || F32WasmMemoryViewCache.buffer !== wasm.memory.buffer) {
        F32WasmMemoryViewCache = new Float32Array(wasm.memory.buffer);
    }
    return F32WasmMemoryViewCache;
}
function sliceWASMArrayF32(offset: number, count: number): Float32Array {
    return getF32Memory0().slice(offset / 4, offset / 4 + count)
}

type Option<T> = null | T; 

class SceneResource{
    constructor(handle: number, scene: WASMScene) {
        this.handle = handle;
        this.scene = scene;
    }

    scene: WASMScene
    handle: number

    dispose() {
        // this.scene.
    }
}

export class Geometry extends SceneResource{
}

export class Shading extends SceneResource{
}

export class Scene{
    scene: WASMScene;
    constructor() {
        this.scene = new WASMScene();
    }

    createNode() {
        return new SceneNode(this.scene.create_new_node(), this.scene);
    }

    createRenderObject(geometry: Geometry, shading: Shading) {
        return new RenderObject(this.scene.create_render_object(geometry.handle, shading.handle), this.scene)
    }
}

export class SceneNode extends SceneResource{

    private parent: Option<SceneNode> = null;
    private children: SceneNode[] = [];
    renderObjects: RenderObject[] = [];

    get transform() {
        return sliceWASMArrayF32(this.scene.scene_node_local_matrix_ptr(this.handle), 16);
    }

    getParent() {
        return this.parent;
    }

    add(node: SceneNode) {
        this.scene.node_add_child_by_handle(this.handle, node.handle);
        this.children.push(node);
        node.parent = this;
    }

    remove(node: SceneNode) {
        this.scene.node_remove_child_by_handle(this.handle, node.handle);
        this.children.splice(this.children.indexOf(node), 1);
        node.parent = null;
    }
}

export class RenderObject extends SceneResource{
}