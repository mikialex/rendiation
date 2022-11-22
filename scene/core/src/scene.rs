use crate::*;

use arena::Arena;
use rendiation_algebra::PerspectiveProjection;
use tree::TreeCollection;

pub struct Scene {
  pub background: Option<SceneBackGround>,

  pub default_camera: SceneCamera,
  pub active_camera: Option<SceneCamera>,

  /// All cameras in the scene
  pub cameras: Arena<SceneCamera>,
  /// All lights in the scene
  pub lights: Arena<SceneLightInner>,
  /// All models in the scene
  pub models: Arena<SceneModelType>,

  nodes: Arc<RwLock<SceneNodesCollection>>,
  root: SceneNode,

  pub ext: DynamicExtension,
}

pub enum SceneModelType {
  Common {
    material: SceneMaterial,
    mesh: SceneMesh,
  },
  Foreign(Box<dyn ForeignImplemented>),
}

pub enum SceneMesh {
  Mesh,
  Foreign(Box<dyn ForeignImplemented>),
}

pub enum SceneMaterial {
  Material,
  Foreign(Box<dyn ForeignImplemented>),
}

pub trait ForeignImplemented: std::any::Any {}

#[derive(Default)]
pub struct DynamicExtension {
  inner: HashMap<std::any::TypeId, Box<dyn std::any::Any>>,
}

pub struct SceneNodesCollection {
  pub(crate) root: SceneNodeHandle,
  pub(crate) nodes: TreeCollection<SceneNodeData>,
}

impl Default for SceneNodesCollection {
  fn default() -> Self {
    let root = SceneNodeData::default();
    let mut nodes = TreeCollection::default();
    let root = nodes.create_node(root);
    Self { root, nodes }
  }
}

impl Scene {
  pub fn root(&self) -> &SceneNode {
    &self.root
  }
  pub fn new() -> Self {
    let nodes: Arc<RwLock<SceneNodesCollection>> = Default::default();

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
    let root = nodes.root;
    nodes.nodes.traverse_mut_pair(root, |parent, this| {
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
