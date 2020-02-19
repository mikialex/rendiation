use crate::renderer::WGPURenderer;
use crate::renderer::render_pass::WGPURenderPass;

pub struct Scene {
    // geometries: 
}

pub trait Renderable {
    fn prepare(&mut self, renderer: &mut WGPURenderer);
    fn render(&self, pass: &WGPURenderPass);
}

// pub struct RenderObject {
//     geometry: StandardGeometry,
//     shading: 
// }