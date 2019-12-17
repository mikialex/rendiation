import { WasmSceneGraph, WasmSceneNode } from "../client/wasm-scene-graph";
import * as THREE from './node_modules/three/src/Three' // i have no idea

export function benchMatrix() {
    function buildWasmSceneGraph(scene: WasmSceneGraph, childrenCount: number, depth: number) {
        let count = 0;
        function addChildren(node: WasmSceneNode, d: number) {
          if (d > 0) {
            for (let i = 0; i < childrenCount; i++) {
              const child = scene.createNewNode();
              // child.positionX = Math.random();
              // child.rotationX = Math.random();
              // child.scaleZ = Math.random();
              count++;
              node.add(child);
              addChildren(child, d - 1);
            }
          }
        }
        addChildren(scene.root, depth)
      }
      
      function buildTHREEScene(scene: THREE.Scene, childrenCount: number, depth: number) {
        let count = 0;
        function addChildren(node: THREE.Object3D, d: number) {
          if (d > 0) {
            for (let i = 0; i < childrenCount; i++) {
              const child = new THREE.Object3D()
              child.matrixAutoUpdate = false;
              // child.position.x = Math.random();
              // child.rotation.y = Math.random();
              // child.scale.z= Math.random();
              count++;
              node.add(child);
              addChildren(child, d - 1);
            }
          }
        }
        addChildren(scene, depth);
        console.log(`${count} has add to scene (object 3d)`)
      }
      
      
      
      function output(result: number[]) {
        let sum = 0;
        result.forEach(re => {
          sum += re;
        })
        // console.log(result)
        console.log("avg:" + sum / result.length);
      }
      
      
      /// benching wasm2
      console.log('===========')
      const wasmSceneGraph = new WasmSceneGraph();
      buildWasmSceneGraph(wasmSceneGraph, 6, 6);
      console.log("wasm graph");
      let wasmresult2 = []
      for (let i = 0; i < 50; i++) {
        let t = performance.now();
        wasmSceneGraph.perf_matrix();
        t = performance.now() - t;
        wasmresult2.push(t);
      }
      wasmresult2 = wasmresult2.slice(3);
      output(wasmresult2)
      
      
      
      
      
      
      /// benching threejs
      console.log('===========')
      const scenethree = new THREE.Scene();
      scenethree.matrixAutoUpdate = false;
      buildTHREEScene(scenethree, 6, 6);
      
      console.log("three");
      let threeresult = []
      for (let i = 0; i < 50; i++) {
        let t = performance.now();
        scenethree.updateWorldMatrix(true, true);
        t = performance.now() - t;
        threeresult.push(t);
      }
      threeresult = threeresult.slice(3);
      output(threeresult)
      
      
}