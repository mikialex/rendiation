use crate::geometry::StandardGeometry;
use crate::renderer::WGPURenderer;
use crate::renderer::render_pass::WGPURenderPass;

pub struct Scene {
    geometries: Vec<StandardGeometry>
}

pub trait Renderable {
    fn prepare(&mut self, renderer: &mut WGPURenderer);
    fn render(&self, pass: &WGPURenderPass);
}

// pub struct RenderObject {
//     geometry: StandardGeometry,
//     shading: 
// }

pub trait Background: Renderable {

}

pub struct Sky{
    geometry: StandardGeometry,
}