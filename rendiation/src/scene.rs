use crate::renderer::WGPURenderer;
use crate::renderer::render_pass::WGPURenderPass;

struct Scene {

}

trait Renderable {
    fn prepare(&mut self, renderer: &mut WGPURenderer);
    fn render(&self, pass: &WGPURenderPass);
}