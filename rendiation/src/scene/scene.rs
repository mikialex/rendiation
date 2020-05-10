use super::{
  background::{Background, SolidBackground},
  culling::Culler,
  node::{RenderData, RenderObject, SceneNode},
  render_list::RenderList,
  resource::ResourceManager,
};
use crate::{render_target::RenderTarget, WGPURenderer};
use generational_arena::{Arena, Index};
use rendiation_render_entity::{Camera, PerspectiveCamera};
use std::cell::RefCell;

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
  pub resources: ResourceManager,

  scene_raw_list: RefCell<RenderList>,
  culled_list: RefCell<RenderList>,
  culler: Culler,
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
      resources: ResourceManager::new(),
      scene_raw_list: RefCell::new(RenderList::new()),
      culled_list: RefCell::new(RenderList::new()),
      culler: Culler::new(),
    }
  }

  pub fn set_new_active_camera(&mut self, camera: impl Camera + 'static) -> Index {
    let boxed = Box::new(camera);
    let index = self.cameras.insert(boxed);
    self.active_camera_index = index;
    index
  }

  pub fn get_active_camera_mut(&mut self) -> &mut Box<dyn Camera> {
    self.cameras.get_mut(self.active_camera_index).unwrap()
  }

  pub fn get_active_camera_mut_downcast<T: 'static>(&mut self) -> &mut T {
    self
      .cameras
      .get_mut(self.active_camera_index)
      .unwrap()
      .as_any_mut()
      .downcast_mut::<T>()
      .unwrap()
  }

  pub fn get_root_node_mut(&mut self) -> &mut SceneNode {
    self.get_node_mut(self.root)
  }

  pub fn get_node(&self, index: Index) -> &SceneNode {
    self.nodes.get(index).unwrap()
  }

  pub fn get_root(&self) -> &SceneNode {
    self.nodes.get(self.root).unwrap()
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

  pub fn create_render_object(&mut self, geometry_index: Index, shading_index: Index) -> Index {
    let obj = RenderObject {
      shading_index,
      geometry_index,
    };
    self.render_objects.insert(obj)
  }

  pub fn delete_render_object(&mut self, index: Index) {
    self.render_objects.remove(index);
  }

  pub fn prepare(&mut self, renderer: &mut WGPURenderer) {
    let mut ctx = ScenePrepareCtx {};
    self
      .renderables_dynamic
      .iter_mut()
      .for_each(|(_, renderable)| {
        renderable.prepare(renderer, &mut ctx);
      });

    // todo hierarchy updating;

    // prepare render list;
    let mut render_list = self.scene_raw_list.borrow_mut();
    render_list.clear();
    for (index, n) in &self.nodes {
      if n.render_objects.len() > 0 {
        render_list.push(index);
      }
    }
    // todo
    // self.get_root().traverse(self, |node|{
    //   render_list.push(node.get_id());
    // });
  }

  pub fn render(&self, target: &RenderTarget, renderer: &mut WGPURenderer) {
    let mut pass = target
      .create_render_pass_builder()
      .first_color(|c| c.load_with_clear((0.1, 0.2, 0.3).into(), 1.0).ok())
      .create(&mut renderer.encoder);

    // pass.use_viewport(&state.viewport);

    for node_id in &self.scene_raw_list.borrow().render_objects {
      let node = self.nodes.get(*node_id).unwrap();
      for render_obj_id in &node.render_objects {
        let render_obj = self.render_objects.get(*render_obj_id).unwrap();
        render_obj.render(&mut pass, self);
      }
    }
  }

  pub fn execute_culling(&mut self) {
    let from = self.scene_raw_list.borrow_mut();
    let mut to = self.culled_list.borrow_mut();
    to.clear();

    for node_id in &from.render_objects {
      // let node = self.nodes.get(*node_id).unwrap();
      // for render_obj_id in &node.render_objects {
      if self.culler.test_is_visible(*node_id, self) {
        to.push(*node_id);
      }
      // }
    }
  }
}

pub struct ScenePrepareCtx {}
