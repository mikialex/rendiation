import { SceneGraph } from "../pkg/wasm_scene";

export class WasmSceneGraph{
    constructor() {
        this.wasmScene = SceneGraph.new();
        this.root = new WasmSceneNode(this, 0);
    }
    free() {
        this.wasmScene.free();
    }

    useProjection(p: Float32Array, inv: Float32Array) {
        this.wasmScene.update_camera(p, inv);
    }

    private wasmScene: SceneGraph;
    getWasm() { return this.wasmScene;}

    createNewNode(): WasmSceneNode {
        const index = this.wasmScene.create_new_node();
        const node = new WasmSceneNode(this, index);
        return node;
    }

    createNewBuffer(data: Float32Array, stride: number) {
        const index = this.wasmScene.create_new_buffer_data(data, stride);
        const buffer = new BufferData(this, index);
        return buffer;
    }

    createNewIndexBuffer(data: Uint16Array, stride: number) {
        const index = this.wasmScene.create_new_index_buffer_data(data, stride);
        const buffer = new BufferData(this, index);
        return buffer;
    }

    createNewGeometry(indexBuffer: BufferData | null, positionBuffer: BufferData) {
        let indexID = null;
        if (indexBuffer !== null) {
            indexID = indexBuffer.index
        }
        const index = this.wasmScene.create_geometry(indexID, positionBuffer.index);
        const geometry = new Geometry(this, index);
        return geometry;
    }

    createShading(keyName: string){
        const index = this.wasmScene.create_new_shading(keyName);
        const shading = new Shading(this, index);
        return shading;
    }

    createRenderObject(s: Shading, g: Geometry) {
        const index = this.wasmScene.create_render_data(g.index, s.index);
        const obj = new RenderObject(this, index, g, s);
        return obj;
    }


    perf_matrix() {
        this.wasmScene.update_all_world_matrix();
    }

    readonly root: WasmSceneNode
}

export class WASMIndexedObject{
    constructor(scene: WasmSceneGraph, index: number) {
        this.index = index;
        this.scene = scene;
    }
    readonly index: number;
    readonly scene: WasmSceneGraph
}

export class Geometry extends WASMIndexedObject{
}

export class BufferData extends WASMIndexedObject{
}

export class Shading  extends WASMIndexedObject{
}

export class RenderObject extends WASMIndexedObject{
    constructor(scene: WasmSceneGraph, index: number, g: Geometry, s: Shading) {
        super(scene, index)
        this.geometry = g
        this.shading = s
    }
    shading: Shading
    geometry: Geometry
}

export class WasmSceneNode extends WASMIndexedObject{
    
    readonly index: number;
    readonly scene: WasmSceneGraph
    private parent: WasmSceneNode | null = null;
    children: WasmSceneNode[] = [];

    setPosition(x: number, y: number, z: number) {
        this.scene.getWasm().set_node_position(this.index, x, y, z);
    }

    setRotation(x: number, y: number, z: number, w: number) {
        this.scene.getWasm().set_node_quaternion(this.index, x, y, z, w);
    }

    private renderData: RenderObject = null;
    get renderObject() {
        return this.renderData;
    }

    set renderObject(obj: RenderObject) {
        this.renderData = obj;
        this.scene.getWasm().set_render_descriptor(
            obj.index, this.index
        )
    }

    add(node: WasmSceneNode) {
        if (node.parent !== null) {
            throw 'before add to another, remove first'
        }
        if (node.scene !== this.scene) {
            throw 'only node in same scene can add together'
        }

        this.children.push(node);
        node.parent = this;
        this.scene.getWasm().add(this.index, node.index);
    }

    remove(node: WasmSceneNode) {
        const index = this.children.indexOf(node);
        if ( index === -1) {
            throw 'remove a not exist node'
        }
        node.parent = null;
        this.children.splice(index, 1);
        this.scene.getWasm().remove(node.index);
    }
}