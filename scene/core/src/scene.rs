use crate::*;

use arena::Arena;
use arena_tree::{ArenaTree, NextTraverseVisit};
use rendiation_algebra::PerspectiveProjection;

pub trait SceneContent {
  type BackGround;
  type Model;
  type Light;
  type Texture2D;
  type TextureCube;
  type SceneExt: Default;
}

pub struct Scene<S: SceneContent> {
  pub background: Option<S::BackGround>,

  pub default_camera: SceneCamera,
  pub active_camera: Option<SceneCamera>,

  /// All cameras in the scene
  pub cameras: Arena<SceneCamera>,
  /// All lights in the scene
  pub lights: Arena<SceneLight<S>>,
  /// All models in the scene
  pub models: Vec<S::Model>,

  nodes: Arc<RwLock<ArenaTree<SceneNodeData>>>,
  root: SceneNode,

  pub extension: S::SceneExt,
}

impl<S: SceneContent> Scene<S> {
  pub fn root(&self) -> &SceneNode {
    &self.root
  }
  pub fn new() -> Self {
    let nodes: Arc<RwLock<ArenaTree<SceneNodeData>>> = Default::default();

    let root = SceneNode::from_root(nodes.clone());

    let default_camera = PerspectiveProjection::default();
    let camera_node = root.create_child();
    let default_camera = SceneCamera::new(default_camera, camera_node);

    Self {
      nodes,
      root,
      background: None,
      default_camera,
      cameras: Arena::new(),
      lights: Arena::new(),
      models: Vec::new(),

      active_camera: None,
      extension: Default::default(),
    }
  }

  pub fn maintain(&mut self) {
    let mut nodes = self.nodes.write().unwrap();
    let root = nodes.root();
    nodes.traverse_mut(root, &mut Vec::new(), |this, parent| {
      let node_data = this.data_mut();
      node_data.hierarchy_update(parent.map(|p| p.data()).map(|d| d.deref()));
      NextTraverseVisit::VisitChildren
    });
  }
}

impl<S: SceneContent> Default for Scene<S> {
  fn default() -> Self {
    Self::new()
  }
}
