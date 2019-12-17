

import * as wasm from '../../pkg/wasm_scene_bg.wasm';

let memory = wasm.memory;
let cachegetFloat32Memory = null;
function getFloat32Memory() {
    if (cachegetFloat32Memory === null || cachegetFloat32Memory.buffer !== memory.buffer) {
        cachegetFloat32Memory = new Float32Array(memory.buffer);
    }
    return cachegetFloat32Memory;
}

export function makeBuffer(size) {
    return new Float32Array(size);
}

export function copyBuffer(buffer, offset, size) {
    const wasm = getFloat32Memory();
    for (let i = 0; i < size; i++) {
        buffer[i] = wasm[offset/4 + i];
    }
}

export function uploadMatrix4f(gl, location, buffer) {
    gl.uniformMatrix4fv(location, false, buffer); 
}