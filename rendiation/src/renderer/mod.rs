use std::collections::HashMap;

pub struct WGPURenderer {
  pipelines: HashMap<String, WGPUPipeline>,
}

pub struct WGPUPipeline {
  pipeline: wgpu::RenderPipeline,
}

// use rendiation_render_entity::*;
// impl Shading<WGPURenderer> for DynamicShading {
//   fn get_index(&self) -> usize {

//   }
//   fn get_vertex_str(&self) -> &str {}
//   fn get_fragment_str(&self) -> &str {}
//   fn make_gpu_port(&self, renderer: &WGPURenderer) -> Rc<dyn ShadingGPUPort<WGPURenderer>> {}
// }
