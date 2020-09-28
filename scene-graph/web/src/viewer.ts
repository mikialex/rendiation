import { Camera } from "../node_modules/three/src/Three";
import { Scene } from "./scene";

export class Viewer {
    canvas: HTMLCanvasElement;
    resource: ResourceManagerWASM;
    scene: Scene;
    camera: Camera;
}


class Shading {

}