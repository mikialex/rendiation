import { WASMScene, AnyHandle } from "../../pkg/rendiation_scenegraph";

export class Scene{
    scene: WASMScene;
    constructor() {
        this.scene = new WASMScene();
    }

    createNode() {
        return new SceneNode(this.scene.create_new_node(), this.scene);
    }
}


export class SceneNode{
    constructor(handle: AnyHandle, scene: WASMScene) {
        this.handle = handle;
        this.scene = scene;
    }

    scene: WASMScene
    handle: AnyHandle

    children: SceneNode[] = [];
    renderObjects: RenderObject[] = [];
}

export class RenderObject{
    scene: WASMScene
    handle: AnyHandle

}