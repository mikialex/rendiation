use std::ops::Deref;

use crate::*;

use arena::{Arena, Handle};
use tree::*;

pub type SceneLightHandle = Handle<SceneLight>;
pub type SceneModelHandle = Handle<SceneModel>;
pub type SceneCameraHandle = Handle<SceneCamera>;

pub struct SceneInner {
  pub background: Option<SceneBackGround>,

  pub default_camera: SceneCamera,
  pub active_camera: Option<SceneCamera>,

  pub content: SceneItemRef<SceneContentCollection>,

  pub nodes: SceneNodeCollection,
  root: SceneNode,

  pub ext: DynamicExtension,
}

#[derive(Default)]
pub struct SceneContentCollection {
  /// All cameras in the scene
  pub cameras: Arena<SceneCamera>,
  /// All lights in the scene
  pub lights: Arena<SceneLight>,
  /// All models in the scene
  pub models: Arena<SceneModel>,
}

#[allow(non_camel_case_types)]
#[derive(Clone)]
pub enum SceneContentCollectionDelta {
  cameras(DeltaOf<Arena<SceneCamera>>),
  lights(DeltaOf<Arena<SceneLight>>),
  models(DeltaOf<Arena<SceneModel>>),
}

impl IncrementalBase for SceneContentCollection {
  type Delta = SceneContentCollectionDelta;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    use SceneContentCollectionDelta::*;
    self.cameras.expand(|d| cb(cameras(d)));
    self.lights.expand(|d| cb(lights(d)));
    self.models.expand(|d| cb(models(d)));
  }
}

#[derive(Default)]
pub struct SceneNodeCollection {
  pub inner: SharedTreeCollection<ReactiveTreeCollection<SceneNodeData, SceneNodeDataImpl>>,
}

impl IncrementalBase for SceneNodeCollection {
  type Delta = TreeMutation<SceneNodeDataImpl>;

  fn expand(&self, cb: impl FnMut(Self::Delta)) {
    self.inner.visit_inner(|tree| {
      tree
        .inner
        .expand_with_mapping(|node| node.deref().clone(), cb)
    });
  }
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
  nodes(DeltaOf<SceneNodeCollection>),
}

impl IncrementalBase for SceneInner {
  type Delta = SceneInnerDelta;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    use SceneInnerDelta::*;
    self.background.expand(|d| cb(background(d)));
    self.default_camera.expand(|d| cb(default_camera(d)));
    self.active_camera.expand(|d| cb(active_camera(d)));

    let content = self.content.read();
    content.expand(|d| match d {
      SceneContentCollectionDelta::cameras(d) => cb(cameras(d)),
      SceneContentCollectionDelta::lights(d) => cb(lights(d)),
      SceneContentCollectionDelta::models(d) => cb(models(d)),
    });

    self.ext.expand(|d| cb(ext(d)));
    self.nodes.expand(|d| cb(nodes(d)));
  }
}

impl SceneInner {
  pub fn root(&self) -> &SceneNode {
    &self.root
  }
  pub fn new() -> (Scene, SceneNodeDeriveSystem) {
    let nodes: SceneNodeCollection = Default::default();
    let system = SceneNodeDeriveSystem::new(&nodes);

    let content = SceneContentCollection::default().into_ref();

    let root = SceneNode::from_new_root(nodes.inner.clone(), content.clone());

    let default_camera = PerspectiveProjection::default();
    let camera_node = root.create_child();
    let default_camera = SceneCamera::create_camera(default_camera, camera_node);

    let scene = Self {
      nodes,
      root,
      background: None,
      default_camera,
      content,
      active_camera: None,
      ext: Default::default(),
    }
    .into_ref();

    // forward the inner change to outer
    let scene_clone = scene.clone();
    let scene_clone2 = scene.clone();
    let s = scene.read();
    s.content.read().delta_source.on(move |d| {
      use SceneContentCollectionDelta::*;
      let delta = match d.delta.clone() {
        cameras(d) => SceneInnerDelta::cameras(d),
        lights(d) => SceneInnerDelta::lights(d),
        models(d) => SceneInnerDelta::models(d),
      };
      scene_clone.mutate(|mut scene| scene.trigger_manual(|_| delta));
      false
    });

    s.nodes.inner.visit_inner(move |tree| {
      tree.source.on(move |d| {
        scene_clone2
          .mutate(|mut scene| scene.trigger_manual(|_| SceneInnerDelta::nodes(d.clone())));
        false
      })
    });

    drop(s);

    (scene, system)
  }

  pub fn get_active_camera(&self) -> &SceneCamera {
    self.active_camera.as_ref().unwrap()
  }
}

pub type Scene = SceneItemRef<SceneInner>;

impl Scene {
  pub fn compute_full_derived(&self) -> ComputedDerivedTree<SceneNodeDerivedData> {
    self.visit(|t| {
      t.nodes
        .inner
        .visit_inner(|t| ComputedDerivedTree::compute_from(&t.inner))
    })
  }

  // // todo improves
  // pub fn insert_model(&self, model: SceneModel) -> SceneModelHandle {
  //   let mut result = None;
  //   self.mutate(|mut scene| {
  //     scene.trigger_manual(|scene| {
  //       let handle = scene.models.insert(model.clone());
  //       result = handle.into();
  //       let delta = ArenaDelta::Insert((model, handle));
  //       SceneInnerDelta::models(delta)
  //     });
  //   });
  //   result.unwrap()
  // }

  // pub fn insert_light(&self, light: SceneLight) -> SceneLightHandle {
  //   let mut result = None;
  //   self.mutate(|mut scene| {
  //     scene.trigger_manual(|scene| {
  //       let handle = scene.lights.insert(light.clone());
  //       result = handle.into();
  //       let delta = ArenaDelta::Insert((light, handle));
  //       SceneInnerDelta::lights(delta)
  //     });
  //   });
  //   result.unwrap()
  // }

  // pub fn insert_camera(&self, camera: SceneCamera) -> SceneCameraHandle {
  //   let mut result = None;
  //   self.mutate(|mut scene| {
  //     scene.trigger_manual(|scene| {
  //       let handle = scene.cameras.insert(camera.clone());
  //       result = handle.into();
  //       let delta = ArenaDelta::Insert((camera, handle));
  //       SceneInnerDelta::cameras(delta)
  //     });
  //   });
  //   result.unwrap()
  // }

  pub fn set_active_camera(&self, camera: Option<SceneCamera>) {
    self.mutate(|mut scene| {
      scene.trigger_manual(|scene| {
        scene.active_camera = camera.clone();
        let camera = camera.map(MaybeDelta::All);
        SceneInnerDelta::active_camera(camera)
      })
    })
  }

  pub fn set_background(&self, background: Option<SceneBackGround>) {
    self.mutate(|mut scene| {
      scene.trigger_manual(|scene| {
        scene.background = background.clone();
        let background = background.map(MaybeDelta::All);
        SceneInnerDelta::background(background)
      });
    })
  }
}
