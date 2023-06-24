use std::ops::Deref;

use arena::{Arena, ArenaDelta, Handle};
use tree::*;

use crate::*;

pub type SceneLightHandle = Handle<SceneLight>;
pub type SceneModelHandle = Handle<SceneModel>;
pub type SceneCameraHandle = Handle<SceneCamera>;

pub struct SceneInner {
  pub background: Option<SceneBackGround>,

  pub _default_camera: SceneCamera,
  pub active_camera: Option<SceneCamera>,

  /// All cameras in the scene
  pub cameras: Arena<SceneCamera>,
  /// All lights in the scene
  pub lights: Arena<SceneLight>,
  /// All models in the scene
  pub models: Arena<SceneModel>,

  pub nodes: SceneNodeCollection,
  root: SceneNode,

  pub ext: DynamicExtension,
}

#[derive(Clone)]
pub struct SceneNodeCollection {
  pub inner: SceneNodeCollectionInner,
  pub scene_guid: usize,
}
pub type SceneNodeCollectionInner = SharedTreeCollection<
  ReactiveTreeCollection<RwLock<TreeCollection<SceneNodeData>>, SceneNodeDataImpl>,
>;

impl SceneNodeCollection {
  pub fn create_node(&self, data: SceneNodeDataImpl) -> SceneNode {
    SceneNode::create_new(self.inner.clone(), data, self.scene_guid)
  }
}

impl IncrementalBase for SceneNodeCollection {
  type Delta = TreeMutation<SceneNodeDataImpl>;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    let tree = self.inner.inner().inner.read().unwrap();
    tree.expand_with_mapping(|node| node.deref().clone(), |d| cb(d.into()));
  }
}

impl SceneInner {
  pub fn root(&self) -> &SceneNode {
    &self.root
  }
  pub fn new() -> (Scene, SceneNodeDeriveSystem) {
    let nodes = SceneNodeCollection {
      inner: Default::default(),
      scene_guid: 0, // set later
    };
    let system = SceneNodeDeriveSystem::new(&nodes);

    let root = nodes.create_node(Default::default());

    let default_camera = PerspectiveProjection::default();
    let default_camera = CameraProjector::Perspective(default_camera);
    let camera_node = root.create_child();
    let _default_camera = SceneCamera::create(default_camera, camera_node);

    let scene = Self {
      nodes,
      root,
      background: None,
      _default_camera,
      cameras: Arena::new(),
      lights: Arena::new(),
      models: Arena::new(),
      active_camera: None,
      ext: Default::default(),
    }
    .into_ref();

    // forward the inner change to outer
    let scene_source_clone = scene.read().delta_source.clone();
    let scene_id = scene.guid();

    let s = scene.read();
    s.nodes.inner.inner().source.on(move |d| {
      scene_source_clone.emit(&SceneInnerDelta::nodes(d.clone()));
      false
    });
    drop(s);

    let mut s = scene.write_unchecked();
    s.mutate_unchecked(|s| {
      s.nodes.scene_guid = scene_id;
      s.root.scene_id = scene_id;
    });
    drop(s);

    (scene, system)
  }

  pub fn get_active_camera(&self) -> &SceneCamera {
    self.active_camera.as_ref().unwrap()
  }
}

pub type Scene = SceneItemRef<SceneInner>;

fn arena_insert<T: IncrementalBase>(
  arena: &mut Arena<SceneItemRef<T>>,
  item: SceneItemRef<T>,
) -> (ArenaDelta<SceneItemRef<T>>, Handle<SceneItemRef<T>>) {
  let handle = arena.insert(item.clone());
  let delta = ArenaDelta::Insert((item, handle));
  (delta, handle)
}

fn arena_remove<T: IncrementalBase>(
  arena: &mut Arena<SceneItemRef<T>>,
  handle: Handle<SceneItemRef<T>>,
) -> ArenaDelta<SceneItemRef<T>> {
  arena.remove(handle);
  ArenaDelta::Remove(handle)
}

impl Scene {
  pub fn create_root_child(&self) -> SceneNode {
    let root = self.read().root().clone(); // avoid dead lock
    root.create_child()
  }

  pub fn compute_full_derived(&self) -> ComputedDerivedTree<SceneNodeDerivedData> {
    self.visit(|t| {
      let tree = t.nodes.inner.inner().inner.read().unwrap();
      ComputedDerivedTree::compute_from(&tree)
    })
  }

  pub fn insert_model(&self, model: SceneModel) -> SceneModelHandle {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      let (delta, handle) = arena_insert(&mut s.models, model);
      scene.trigger_change_but_not_apply(delta.wrap(SceneInnerDelta::models));
      handle
    })
  }
  pub fn remove_model(&self, model: SceneModelHandle) {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      let delta = arena_remove(&mut s.models, model);
      scene.trigger_change_but_not_apply(delta.wrap(SceneInnerDelta::models));
    })
  }

  pub fn insert_light(&self, light: SceneLight) -> SceneLightHandle {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      let (delta, handle) = arena_insert(&mut s.lights, light);
      scene.trigger_change_but_not_apply(delta.wrap(SceneInnerDelta::lights));
      handle
    })
  }
  pub fn remove_light(&self, light: SceneLightHandle) {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      let delta = arena_remove(&mut s.lights, light);
      scene.trigger_change_but_not_apply(delta.wrap(SceneInnerDelta::lights));
    })
  }

  pub fn insert_camera(&self, camera: SceneCamera) -> SceneCameraHandle {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      let (delta, handle) = arena_insert(&mut s.cameras, camera);
      scene.trigger_change_but_not_apply(delta.wrap(SceneInnerDelta::cameras));
      handle
    })
  }
  pub fn remove_camera(&self, camera: SceneCameraHandle) {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      let delta = arena_remove(&mut s.cameras, camera);
      scene.trigger_change_but_not_apply(delta.wrap(SceneInnerDelta::cameras));
    })
  }

  pub fn set_active_camera(&self, camera: Option<SceneCamera>) {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      s.active_camera = camera.clone();
      let delta = camera
        .map(MaybeDelta::All)
        .wrap(SceneInnerDelta::active_camera);
      scene.trigger_change_but_not_apply(delta);
    })
  }

  pub fn set_background(&self, background: Option<SceneBackGround>) {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      s.background = background.clone();
      let delta = background
        .map(MaybeDelta::All)
        .wrap(SceneInnerDelta::background);
      scene.trigger_change_but_not_apply(delta);
    })
  }

  pub fn update_ext(&self, delta: DeltaOf<DynamicExtension>) {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      s.ext.apply(delta.clone()).unwrap();
      scene.trigger_change_but_not_apply(delta.wrap(SceneInnerDelta::ext));
    })
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

impl std::fmt::Debug for SceneInnerDelta {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::background(_) => f.debug_tuple("background").finish(),
      Self::default_camera(_) => f.debug_tuple("default_camera").finish(),
      Self::active_camera(_) => f.debug_tuple("active_camera").finish(),
      Self::cameras(_) => f.debug_tuple("cameras").finish(),
      Self::lights(_) => f.debug_tuple("lights").finish(),
      Self::models(_) => f.debug_tuple("models").finish(),
      Self::ext(_) => f.debug_tuple("ext").finish(),
      Self::nodes(_) => f.debug_tuple("nodes").finish(),
    }
  }
}

impl IncrementalBase for SceneInner {
  type Delta = SceneInnerDelta;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    use SceneInnerDelta::*;
    self.nodes.expand(|d| cb(nodes(d)));
    self.background.expand(|d| cb(background(d)));
    self.active_camera.expand(|d| cb(active_camera(d)));
    self.cameras.expand(|d| cb(cameras(d)));
    self.lights.expand(|d| cb(lights(d)));
    self.models.expand(|d| cb(models(d)));
    self.ext.expand(|d| cb(ext(d)));
  }
}
