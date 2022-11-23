use crate::*;

use arena::{Arena, Handle};
use incremental::{Incremental, SimpleMutator};
use rendiation_algebra::PerspectiveProjection;
use tree::TreeCollection;

pub type SceneModelHandle = Handle<SceneModelType>;
pub type SceneCameraHandle = Handle<SceneCamera>;

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

pub trait ForeignImplemented: std::any::Any + dyn_clone::DynClone + Send + Sync {
  fn as_any(&self) -> &dyn std::any::Any;
  fn as_mut_any(&mut self) -> &mut dyn std::any::Any;
}

#[derive(Default, Clone, Debug)]
pub struct DynamicExtension {
  inner: HashMap<std::any::TypeId, std::rc::Rc<dyn std::any::Any>>,
}

impl Incremental for DynamicExtension {
  type Delta = ();

  type Error = ();

  type Mutator<'a> = SimpleMutator<'a, Self>
  where
    Self: 'a;

  fn create_mutator<'a>(
    &'a mut self,
    collector: &'a mut dyn FnMut(Self::Delta),
  ) -> Self::Mutator<'a> {
    todo!()
  }

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    todo!()
  }

  fn expand(&self, cb: impl FnMut(Self::Delta)) {
    todo!()
  }
}

// impl Incremental for Scene<S> {
//   type Delta;

//   type Error;

//   type Mutator<'a>
//   where
//     Self: 'a;

//   fn create_mutator<'a>(
//     &'a mut self,
//     collector: &'a mut dyn FnMut(Self::Delta),
//   ) -> Self::Mutator<'a> {
//     todo!()
//   }

//   fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
//     todo!()
//   }

//   fn expand(&self, cb: impl FnMut(Self::Delta)) {
//     todo!()
//   }
// }

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
