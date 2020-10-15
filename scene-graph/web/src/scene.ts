import { WASMAttributeBufferF32, WASMAttributeBufferU16, WASMGeometry, WASMScene, WebGLRenderer } from "../../pkg/rendiation_scenegraph";
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

class ViewerResource {
    constructor(handle: number, scene: Viewer) {
        this.handle = handle;
        this.viewer = scene;
    }

    viewer: Viewer
    handle: number

    dispose() {
        // this.scene.
    }
}

export class VertexBuffer extends ViewerResource {
    wasmBuffer: WASMAttributeBufferF32

    constructor(viewer: Viewer, value: WASMAttributeBufferF32) {
        let id = viewer.scene.add_vertex_buffer(value, viewer.renderer);
        super(id, viewer);
    }
}

export class IndexBuffer extends ViewerResource {
    wasmBuffer: WASMAttributeBufferU16

    constructor(viewer: Viewer, value: WASMAttributeBufferU16) {
        let id = viewer.scene.add_index_buffer(value, viewer.renderer);
        super(id, viewer);
    }
}

export class Geometry extends ViewerResource {
    geometry: WASMGeometry;
    constructor(viewer: Viewer, value: WASMGeometry) {
        let id = viewer.scene.add_geometry(value);
        super(id, viewer);
        this.geometry = value;
    }
}

export class Shading<T> extends ViewerResource {
    constructor(viewer: Viewer, value: T) {
        const id = viewer.scene.add_shading();
        super(id, viewer);
    }
    value: T;
}

export class Viewer {
    scene: WASMScene;
    renderer: WebGLRenderer
    constructor(canvas: HTMLCanvasElement) {
        this.scene = new WASMScene();
        this.renderer = new WebGLRenderer(canvas);
    }

    createNode() {
        return new SceneNode(this.scene.create_new_node(), this);
    }

    createDrawcall(geometry: Geometry, shading: Shading<any>): Drawcall {
        return new Drawcall(this.scene.create_drawcall(geometry.handle, shading.handle), this)
    }
}

export class SceneNode extends ViewerResource {

    private parent: Option<SceneNode> = null;
    private children: SceneNode[] = [];
    drawcalls: Drawcall[] = [];

    get transform() {
        return sliceWASMArrayF32(this.viewer.scene.scene_node_local_matrix_ptr(this.handle), 16);
    }

    getParent() {
        return this.parent;
    }

    add(node: SceneNode) {
        this.viewer.scene.node_add_child_by_handle(this.handle, node.handle);
        this.children.push(node);
        node.parent = this;
    }

    remove(node: SceneNode) {
        this.viewer.scene.node_remove_child_by_handle(this.handle, node.handle);
        this.children.splice(this.children.indexOf(node), 1);
        node.parent = null;
    }
}

export class Drawcall extends ViewerResource {
}