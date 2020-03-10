use crate::renderer::texture::WGPUTexture;
use crate::geometry::StandardGeometry;
use crate::renderer::pipeline::WGPUPipeline;
use crate::renderer::WGPURenderer;
use crate::{
  geometry_lib::{sphere_geometry::SphereGeometryParameter, Mesher},
  renderer::render_pass::WGPURenderPass,
  StaticPipelineBuilder,
};
use generational_arena::{Arena, Index};
use rendiation_render_entity::{PerspectiveCamera, Camera};

mod background;
pub use background::*;

pub struct Scene {
  background: Box<dyn Background>,
  active_camera_index: Index,
  cameras: Arena<Box<dyn Camera>>,
  geometries: Arena<StandardGeometry>,
  // renderables: Arena<Box<dyn Renderable>>,
  // nodes: Vec<SceneNode>
  canvas: WGPUTexture
}

impl Scene {
  pub fn new(renderer: &WGPURenderer) -> Self {
    let camera_default = Box::new(PerspectiveCamera::new());
    let mut cameras: Arena<Box<dyn Camera>> = Arena::new();
    let active_camera_index = cameras.insert(camera_default);
    // todo!()
    Self {
      background: Box::new(SolidBackground::new()),
      active_camera_index,
      cameras,
      geometries: Arena::new(),
      canvas: WGPUTexture::new_as_target(&renderer.device, (100, 100))
      // nodes: Vec<SceneNode>
    }
  }

  pub fn prepare(&mut self, renderer: &mut WGPURenderer) {
    // for (_, renderable) in &mut self.renderables {
    //   renderable.prepare(renderer, self);
    // }
  }

  pub fn render() {
    // let mut pass = WGPURenderPass::build()
    //       .output_with_clear(&output.view, (0.1, 0.2, 0.3, 1.0))
    //       .with_depth(state.depth.view())
    //       .create(&mut renderer.encoder);
    //     pass.use_viewport(&state.viewport);

    // for (_, renderable) in &mut self.renderables {
    //   renderable.prepare(renderer, self);
    // }
  }
}

// pub trait SceneNode{
// }

pub trait Renderable {
  fn prepare(&mut self, renderer: &mut WGPURenderer, scene: &mut Scene);
  fn render(&self, renderer: &WGPURenderer, scene: &mut Scene);
}

// pub struct RenderObject {
//     geometry: StandardGeometry,
//     shading:
// }
