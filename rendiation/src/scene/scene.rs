use super::{
  background::{Background, SolidBackground},
  node::{RenderData, RenderObject, SceneNode},
};
use crate::{WGPURenderer, WGPUTexture, GPUGeometry};
use generational_arena::{Arena, Index};
use rendiation_render_entity::{Camera, PerspectiveCamera};

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

    let mut nodes = Arena::new();
    let mut nodes_render_data = Arena::new();

    let root = SceneNode::new();
    let index = nodes.insert(root);
    nodes.get_mut(index).unwrap().set_self_id(index);
    nodes_render_data.insert(RenderData::new());

    Self {
      background: Box::new(SolidBackground::new()),
      active_camera_index,
      cameras,
      render_objects: Arena::new(),
      root: index,
      nodes,
      nodes_render_data,
      renderables_dynamic: Arena::new(),
    }
  }

  pub fn get_root_node_mut(&mut self) -> &mut SceneNode {
    self.get_node_mut(self.root)
  }

  pub fn get_node(&self, index: Index) -> &SceneNode {
    self.nodes.get(index).unwrap()
  }

  pub fn get_node_mut(&mut self, index: Index) -> &mut SceneNode {
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
    self
      .renderables_dynamic
      .iter_mut()
      .for_each(|(i, renderable)| {
        renderable.prepare(renderer, &mut ctx);
      })
  }

  pub fn render(&self, target: &WGPUTexture, renderer: &WGPURenderer) {}
}

pub struct ScenePrepareCtx {}
