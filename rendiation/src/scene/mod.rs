use crate::geometry::StandardGeometry;
use crate::renderer::pipeline::WGPUPipeline;
use crate::renderer::texture::WGPUTexture;
use crate::renderer::WGPURenderer;
use crate::{
  geometry_lib::{sphere_geometry::SphereGeometryParameter, IndexedBufferMesher},
  renderer::render_pass::WGPURenderPass,
  StaticPipelineBuilder,
};
use generational_arena::{Arena, Index};
use rendiation_render_entity::{Camera, PerspectiveCamera};

mod background;
pub use background::*;

pub struct SceneNode {
  parent: Index,
}

pub struct Scene {
  background: Box<dyn Background>,
  active_camera_index: Index,
  cameras: Arena<Box<dyn Camera>>,
  // geometries: Arena<StandardGeometry>,
  renderables: Arena<Box<dyn Renderable>>,
  // nodes: Arena<SceneNode>,
  canvas: WGPUTexture,
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
      // geometries: Arena::new(),
      canvas: WGPUTexture::new_as_target(&renderer, (100, 100)),
      renderables: Arena::new(),
      // nodes: Vec<SceneNode>
    }
  }

  pub fn add_renderable(&mut self, renderable: impl Renderable + 'static) -> Index {
    let boxed = Box::new(renderable);
    self.renderables.insert(boxed)
  }

  pub fn prepare(&mut self, renderer: &mut WGPURenderer) {
    let mut ctx = ScenePrepareCtx {};
    self.renderables.iter_mut().for_each(|(i, renderable)| {
      renderable.prepare(renderer, &mut ctx);
    })
  }

  pub fn render(&self, renderer: &mut WGPURenderer) {
    let mut pass = WGPURenderPass::build()
          .output_with_clear(&self.canvas.view(), (0.1, 0.2, 0.3, 1.0))
          // .with_depth(state.depth.view())
          .create(&mut renderer.encoder);
        // pass.use_viewport(&state.viewport);

    // for (_, renderable) in &self.renderables {
    //   renderable.render(renderer, self);
    // }
  }
}

// pub trait SceneNode{
// }

pub struct ScenePrepareCtx {}

pub trait Renderable {
  fn prepare(&mut self, renderer: &mut WGPURenderer, scene: &mut ScenePrepareCtx);
  fn render(&self, renderer: &WGPURenderer, scene: &Scene);
}

// pub struct RenderObject {
//     geometry: StandardGeometry,
//     shading:
// }
