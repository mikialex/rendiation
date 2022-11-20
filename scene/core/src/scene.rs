use crate::*;

use arena::Arena;
use rendiation_algebra::PerspectiveProjection;
use tree::TreeCollection;

pub trait SceneContent: Clone + Copy + 'static {
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
  pub lights: Arena<S::Light>,
  /// All models in the scene
  pub models: Arena<S::Model>,

  nodes: Arc<RwLock<TreeCollection<SceneNodeData>>>,
  root: SceneNode,

  pub extension: S::SceneExt,
}

impl<S: SceneContent> Scene<S> {
  pub fn root(&self) -> &SceneNode {
    &self.root
  }
  pub fn new() -> Self {
    let nodes: Arc<RwLock<TreeCollection<SceneNodeData>>> = Default::default();

    let root = SceneNode::from_root(nodes.clone());

    let default_camera = PerspectiveProjection::default();
    let camera_node = root.create_child();
    let default_camera = SceneCamera::create_camera(default_camera, camera_node);

    Self {
      nodes,
      root,
      background: None,
      default_camera,
      cameras: Arena::new(),
      lights: Arena::new(),
      models: Arena::new(),

      active_camera: None,
      extension: Default::default(),
    }
  }

  pub fn get_active_camera(&self) -> &SceneCamera {
    self.active_camera.as_ref().unwrap()
  }

  pub fn maintain(&mut self) {
    let mut nodes = self.nodes.write().unwrap();
    let root = self.root.raw_handle();
    nodes.traverse_mut_pair(root, |parent, this| {
      let node_data = this.data_mut();
      node_data.hierarchy_update(Some(parent.data()));
    });
  }
}

impl<S: SceneContent> Default for Scene<S> {
  fn default() -> Self {
    Self::new()
  }
}
