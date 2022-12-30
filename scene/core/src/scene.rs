use crate::*;

use arena::{Arena, ArenaDelta, Handle};
use tree::SharedTreeCollection;

pub type SceneLightHandle = Handle<SceneLight>;
pub type SceneModelHandle = Handle<SceneModel>;
pub type SceneCameraHandle = Handle<SceneCamera>;

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

  nodes: SharedTreeCollection<SceneNodeData>,
  root: SceneNode,

  pub ext: DynamicExtension,
}

#[allow(non_camel_case_types)]
#[derive(Clone)]
pub enum SceneInnerDelta {
  background(DeltaOf<Option<SceneBackGround>>),
  default_camera(DeltaOf<SceneCamera>),
  active_camera(DeltaOf<Option<SceneCamera>>),
  cameras(DeltaOf<Arena<SceneCamera>>),
  lights(DeltaOf<Arena<SceneLight>>),
  models(DeltaOf<Arena<SceneModel>>),
  ext(DeltaOf<DynamicExtension>),
  nodes(DeltaOf<SharedTreeCollection<SceneNodeData>>),
}

impl IncrementalBase for SceneInner {
  type Delta = SceneInnerDelta;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    use SceneInnerDelta::*;
    self.background.expand(|d| cb(background(d)));
    self.default_camera.expand(|d| cb(default_camera(d)));
    self.active_camera.expand(|d| cb(active_camera(d)));
    self.cameras.expand(|d| cb(cameras(d)));
    self.lights.expand(|d| cb(lights(d)));
    self.models.expand(|d| cb(models(d)));
    self.ext.expand(|d| cb(ext(d)));
    self.nodes.expand(|d| cb(nodes(d)));
  }
}

impl SceneInner {
  pub fn root(&self) -> &SceneNode {
    &self.root
  }
  pub fn new() -> Self {
    let nodes: SharedTreeCollection<SceneNodeData> = Default::default();

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
    let mut nodes = self.nodes.inner.write().unwrap();
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
  // todo improves
  pub fn insert_model(&self, model: SceneModel) -> SceneModelHandle {
    let mut result = None;
    self.mutate(|mut scene| {
      scene.trigger_manual(|scene| {
        let handle = scene.models.insert(model.clone());
        result = handle.into();
        let delta = ArenaDelta::Insert((model, handle));
        SceneInnerDelta::models(delta)
      });
    });
    result.unwrap()
  }

  pub fn insert_light(&self, light: SceneLight) -> SceneLightHandle {
    let mut result = None;
    self.mutate(|mut scene| {
      scene.trigger_manual(|scene| {
        let handle = scene.lights.insert(light.clone());
        result = handle.into();
        let delta = ArenaDelta::Insert((light, handle));
        SceneInnerDelta::lights(delta)
      });
    });
    result.unwrap()
  }

  pub fn insert_camera(&self, camera: SceneCamera) -> SceneCameraHandle {
    let mut result = None;
    self.mutate(|mut scene| {
      scene.trigger_manual(|scene| {
        let handle = scene.cameras.insert(camera.clone());
        result = handle.into();
        let delta = ArenaDelta::Insert((camera, handle));
        SceneInnerDelta::cameras(delta)
      });
    });
    result.unwrap()
  }

  pub fn set_active_camera(&self, camera: Option<SceneCamera>) {
    self.mutate(|mut scene| {
      scene.trigger_manual(|scene| {
        scene.active_camera = camera.clone();
        let camera = camera.map(DeltaOrEntire::Entire);
        SceneInnerDelta::active_camera(camera)
      })
    })
  }

  pub fn set_background(&self, background: Option<SceneBackGround>) {
    self.mutate(|mut scene| {
      scene.trigger_manual(|scene| {
        scene.background = background.clone();
        let background = background.map(DeltaOrEntire::Entire);
        SceneInnerDelta::background(background)
      });
    })
  }
}
