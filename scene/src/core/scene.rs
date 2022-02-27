use std::{cell::RefCell, ops::Deref, rc::Rc};

use arena::{Arena, Handle};
use arena_tree::{ArenaTree, ArenaTreeNodeHandle};
use rendiation_algebra::PerspectiveProjection;

use crate::*;

pub type SceneNodeHandle = ArenaTreeNodeHandle<SceneNodeData>;
pub type LightHandle = Handle<Box<dyn Light>>;

pub trait Background {}

impl Background for SolidBackground {}

pub struct Scene {
  pub background: Box<dyn Background>,

  pub default_camera: SceneCamera,
  pub active_camera: Option<SceneCamera>,
  pub cameras: Arena<SceneCamera>,
  pub lights: Arena<SceneLight>,
  pub models: Vec<Box<dyn SceneRenderableRc>>,

  nodes: Rc<RefCell<ArenaTree<SceneNodeData>>>,
  pub root: SceneNode,
  pub resources: GPUResourceCache,
}

impl Scene {
  pub fn new() -> Self {
    let nodes: Rc<RefCell<ArenaTree<SceneNodeData>>> = Default::default();

    let root = SceneNode::from_root(nodes.clone());

    let default_camera = PerspectiveProjection::default();
    let camera_node = root.create_child();
    let default_camera = SceneCamera::new(default_camera, camera_node);

    Self {
      nodes,
      root,
      background: Box::new(SolidBackground::default()),
      default_camera,
      cameras: Arena::new(),
      lights: Arena::new(),
      models: Vec::new(),

      active_camera: None,
      resources: Default::default(),
    }
  }

  pub fn add_model(&mut self, model: impl SceneRenderableRc) {
    self.models.push(Box::new(model));
  }

  pub fn maintain(&mut self) {
    let mut nodes = self.nodes.borrow_mut();
    let root = nodes.root();
    nodes.traverse_mut(root, &mut Vec::new(), |this, parent| {
      let node_data = this.data_mut();
      node_data.hierarchy_update(parent.map(|p| p.data()).map(|d| d.deref()));
      NextTraverseVisit::VisitChildren
    });
    self.resources.cameras.maintain();
  }
}

impl Default for Scene {
  fn default() -> Self {
    Self::new()
  }
}
