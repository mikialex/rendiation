pub struct WGPUTexture{
    gpu_texture: wgpu::Texture,
    
    value: WGPUBuffer,
    width: usize,
    height: usize,


}