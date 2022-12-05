use crate::*;

use arena::{Arena, Handle};
use incremental::Incremental;
use rendiation_algebra::PerspectiveProjection;
use tree::TreeCollection;

pub type SceneModelHandle = Handle<SceneModel>;
pub type SceneCameraHandle = Handle<SceneCamera>;

#[derive(Incremental)]
pub struct Scene {
  pub background: Option<SceneBackGround>,

  pub default_camera: SceneCamera,
  pub active_camera: Option<SceneCamera>,

  /// All cameras in the scene
  pub cameras: Arena<SceneCamera>,
  /// All lights in the scene
  pub lights: Arena<SceneLight>,
  /// All models in the scene
  pub models: Arena<SceneModel>,

  nodes: Arc<RwLock<TreeCollection<SceneNodeData>>>,
  root: SceneNode,

  pub ext: DynamicExtension,
}

impl Scene {
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
      ext: Default::default(),
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

impl Default for Scene {
  fn default() -> Self {
    Self::new()
  }
}
