use crate::*;

use arena::{Arena, ArenaDelta, Handle};
use incremental::Incremental;
use rendiation_algebra::PerspectiveProjection;
use tree::TreeCollection;

pub type SceneModelHandle = Handle<SceneModel>;
pub type SceneCameraHandle = Handle<SceneCamera>;

#[derive(Incremental)]
pub struct SceneInner {
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

impl SceneInner {
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

  pub fn maintain(&self) {
    let mut nodes = self.nodes.write().unwrap();
    let root = self.root.raw_handle();
    nodes.traverse_mut_pair(root, |parent, this| {
      let parent = parent.data();
      let node_data = this.data_mut();
      node_data.mutate(|mut node_data| {
        let new_net = node_data.visible && parent.net_visible;
        if new_net != node_data.net_visible {
          node_data.modify(SceneNodeDataImplDelta::net_visible(new_net))
        }
        if new_net {
          let new_world_matrix = parent.world_matrix * node_data.local_matrix;
          if new_world_matrix != node_data.world_matrix {
            node_data.modify(SceneNodeDataImplDelta::world_matrix(new_world_matrix))
          }
        }
      });
    });
  }
}

impl Default for SceneInner {
  fn default() -> Self {
    Self::new()
  }
}

pub type Scene = SceneItemRef<SceneInner>;

impl Scene {
  pub fn insert_model(&self, model: SceneModel) -> SceneModelHandle {
    self.mutate(|mut scene| {
      let handle = scene.inner.models.insert(model.clone());
      let delta = ArenaDelta::Insert((model, handle));
      let delta = SceneInnerDelta::models(delta);
      scene.trigger_manual(delta);
      handle
    })
  }

  pub fn set_background(&self, background: Option<SceneBackGround>) {
    self.mutate(|mut scene| {
      let background = background.map(|b| DeltaOrEntire::Entire(b));
      scene.modify(SceneInnerDelta::background(background));
    })
  }
}
