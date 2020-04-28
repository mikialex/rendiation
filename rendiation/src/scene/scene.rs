use crate::{WGPURenderer, WGPUTexture};
use rendiation_render_entity::{Camera, PerspectiveCamera};
use generational_arena::{Index, Arena};
use super::{background::Background, node::{SceneNode, RenderObject, RenderData}};

pub trait Renderable {
  fn prepare(&mut self, renderer: &mut WGPURenderer, scene: &mut ScenePrepareCtx);
  fn render(&self, renderer: &WGPURenderer, scene: &Scene);
}

pub struct Scene {
  background: Box<dyn Background>,
  active_camera_index: Index,
  cameras: Arena<Box<dyn Camera>>,

  render_objects: Arena<RenderObject>,

  nodes: Arena<SceneNode>,
  nodes_render_data: Arena<RenderData>,

  renderables_dynamic: Arena<Box<dyn Renderable>>,
  canvas: WGPUTexture,
}

impl Scene {
  pub fn new() -> Self {
    let camera_default = Box::new(PerspectiveCamera::new());
    let mut cameras: Arena<Box<dyn Camera>> = Arena::new();
    let active_camera_index = cameras.insert(camera_default);
    todo!()
    // Self {
    //   background: Box::new(SolidBackground::new()),
    //   active_camera_index,
    //   cameras,
    //   // geometries: Arena::new(),
    //   renderables: Arena::new(),
    //   // nodes: Vec<SceneNode>
    // }
  }

  pub fn add_renderable(&mut self, renderable: impl Renderable + 'static) -> Index {
    let boxed = Box::new(renderable);
    self.renderables_dynamic.insert(boxed)
  }

  pub fn prepare(&mut self, renderer: &mut WGPURenderer) {
    let mut ctx = ScenePrepareCtx {};
    self.renderables_dynamic.iter_mut().for_each(|(i, renderable)| {
      renderable.prepare(renderer, &mut ctx);
    })
  }

  pub fn render(&self, target: &WGPUTexture, renderer: &WGPURenderer) {}
}


pub struct ScenePrepareCtx {}