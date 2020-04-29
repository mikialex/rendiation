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

  root: Index,
  nodes: Arena<SceneNode>,
  pub(crate) nodes_render_data: Arena<RenderData>,

  renderables_dynamic: Arena<Box<dyn Renderable>>,
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

  // pub fn get_camera_mut(&mut self, index: Index) -> {

  // }

  pub fn get_root_node_mut(&mut self) -> &mut SceneNode{
    self.get_node_mut(self.root)
  }

  pub fn get_node(&self, index: Index) -> &SceneNode{
    self.nodes.get(index).unwrap()
  }

  pub fn get_node_mut(&mut self, index: Index) -> &mut SceneNode{
    self.nodes.get_mut(index).unwrap()
  }

  pub fn add_dynamic_renderable(&mut self, renderable: impl Renderable + 'static) -> Index {
    let boxed = Box::new(renderable);
    self.renderables_dynamic.insert(boxed)
  }

  pub fn create_new_node(&mut self) -> &mut SceneNode {
    let new_node = SceneNode::new();
    let index = self.nodes.insert(new_node);
    let new_node = self.nodes.get_mut(index).unwrap().set_self_id(index);
    self.nodes_render_data.insert(RenderData::new());
    new_node
  }

  pub fn free_node(&mut self, index: Index) {
    self.nodes.remove(index);
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