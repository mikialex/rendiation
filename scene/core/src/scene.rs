use arena::{Arena, ArenaDelta, Handle};
use tree::*;

use crate::*;

pub type SceneLightHandle = Handle<SceneLight>;
pub type SceneModelHandle = Handle<SceneModel>;
pub type SceneCameraHandle = Handle<SceneCamera>;

pub type SceneCore = SharedIncrementalSignal<SceneCoreImpl>;

pub struct SceneCoreImpl {
  /// scene environment config, mainly decide background effect.
  pub background: Option<SceneBackGround>,

  /// the rendering camera for main view, should be one of camera in self.cameras
  pub active_camera: Option<SceneCamera>,

  /// All cameras in the scene
  pub cameras: Arena<SceneCamera>,
  /// All lights in the scene
  pub lights: Arena<SceneLight>,
  /// All models in the scene
  pub models: Arena<SceneModel>,

  /// scene tree
  pub nodes: SceneNodeCollection,
  root: SceneNode,

  /// scene level dynamic extension port
  pub ext: DynamicExtension,
}

impl SceneCoreImpl {
  pub fn root(&self) -> &SceneNode {
    &self.root
  }
  fn new() -> (SceneCore, SceneNodeDeriveSystem) {
    let nodes = SceneNodeCollection {
      inner: Default::default(),
      scene_guid: 0, // set later
    };
    let system = SceneNodeDeriveSystem::new(&nodes);

    let root = nodes.create_node(Default::default());

    let scene = Self {
      nodes,
      root,
      background: None,
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
    s.nodes.inner.source.on(move |d| {
      scene_source_clone.emit(&SceneInternalDelta::nodes(d.clone()));
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

fn arena_insert<T: IncrementalBase>(
  arena: &mut Arena<SharedIncrementalSignal<T>>,
  item: SharedIncrementalSignal<T>,
) -> (
  ArenaDelta<SharedIncrementalSignal<T>>,
  Handle<SharedIncrementalSignal<T>>,
) {
  let handle = arena.insert(item.clone());
  let delta = ArenaDelta::Insert((item, handle));
  (delta, handle)
}

fn arena_remove<T: IncrementalBase>(
  arena: &mut Arena<SharedIncrementalSignal<T>>,
  handle: Handle<SharedIncrementalSignal<T>>,
) -> (
  SharedIncrementalSignal<T>,
  ArenaDelta<SharedIncrementalSignal<T>>,
) {
  let removed = arena
    .remove(handle)
    .expect("removed an none exist entity in scene");
  (removed, ArenaDelta::Remove(handle))
}

pub trait SceneCoreExt {
  fn create_root_child(&self) -> SceneNode;
  fn compute_full_derived(&self) -> ComputedDerivedTree<SceneNodeDerivedData>;
  fn insert_model(&self, model: SceneModel) -> SceneModelHandle;
  fn remove_model(&self, model: SceneModelHandle);
  fn insert_light(&self, light: SceneLight) -> SceneLightHandle;
  fn remove_light(&self, light: SceneLightHandle);
  fn insert_camera(&self, camera: SceneCamera) -> SceneCameraHandle;
  fn remove_camera(&self, camera: SceneCameraHandle);
  fn set_active_camera(&self, camera: Option<SceneCamera>);
  fn set_background(&self, background: Option<SceneBackGround>);
  fn update_ext(&self, delta: DeltaOf<DynamicExtension>);
}

impl SceneCoreExt for SceneCore {
  fn create_root_child(&self) -> SceneNode {
    let root = self.read().root().clone(); // avoid dead lock
    root.create_child()
  }

  fn compute_full_derived(&self) -> ComputedDerivedTree<SceneNodeDerivedData> {
    self.visit(|t| {
      let tree = t.nodes.inner.inner.read().unwrap();
      ComputedDerivedTree::compute_from(&tree)
    })
  }

  fn insert_model(&self, model: SceneModel) -> SceneModelHandle {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      let (delta, handle) = arena_insert(&mut s.models, model);

      // todo, the attach index now is exposed to user, but should not..
      let inserted = s.models.get(handle).unwrap();
      assert!(inserted.read().attach_index().is_none());
      let d = SceneModelImplDelta::attach_index(Some(handle.index()).map(MaybeDelta::All));
      inserted.mutate(|mut m| m.modify(d));

      scene.trigger_change_but_not_apply(delta.wrap(SceneInternalDelta::models));
      handle
    })
  }
  fn remove_model(&self, model: SceneModelHandle) {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      let (removed, delta) = arena_remove(&mut s.models, model);

      assert!(removed.read().attach_index().is_some());
      let d = SceneModelImplDelta::attach_index(None.map(MaybeDelta::All));
      removed.mutate(|mut m| m.modify(d));

      scene.trigger_change_but_not_apply(delta.wrap(SceneInternalDelta::models));
    })
  }

  fn insert_light(&self, light: SceneLight) -> SceneLightHandle {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      let (delta, handle) = arena_insert(&mut s.lights, light);

      let inserted = s.lights.get(handle).unwrap();
      assert!(inserted.read().attach_index().is_none());
      let d = SceneLightImplDelta::attach_index(Some(handle.index()).map(MaybeDelta::All));
      inserted.mutate(|mut m| m.modify(d));

      scene.trigger_change_but_not_apply(delta.wrap(SceneInternalDelta::lights));
      handle
    })
  }
  fn remove_light(&self, light: SceneLightHandle) {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      let (removed, delta) = arena_remove(&mut s.lights, light);

      assert!(removed.read().attach_index().is_some());
      let d = SceneLightImplDelta::attach_index(None.map(MaybeDelta::All));
      removed.mutate(|mut m| m.modify(d));

      scene.trigger_change_but_not_apply(delta.wrap(SceneInternalDelta::lights));
    })
  }

  fn insert_camera(&self, camera: SceneCamera) -> SceneCameraHandle {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      let (delta, handle) = arena_insert(&mut s.cameras, camera);

      let inserted = s.cameras.get(handle).unwrap();
      assert!(inserted.read().attach_index().is_none());
      let d = SceneCameraImplDelta::attach_index(Some(handle.index()).map(MaybeDelta::All));
      inserted.mutate(|mut m| m.modify(d));

      scene.trigger_change_but_not_apply(delta.wrap(SceneInternalDelta::cameras));
      handle
    })
  }
  fn remove_camera(&self, camera: SceneCameraHandle) {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      let (removed, delta) = arena_remove(&mut s.cameras, camera);

      assert!(removed.read().attach_index().is_some());
      let d = SceneCameraImplDelta::attach_index(None.map(MaybeDelta::All));
      removed.mutate(|mut m| m.modify(d));

      scene.trigger_change_but_not_apply(delta.wrap(SceneInternalDelta::cameras));
    })
  }

  fn set_active_camera(&self, camera: Option<SceneCamera>) {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      s.active_camera = camera.clone();
      let delta = camera
        .map(MaybeDelta::All)
        .wrap(SceneInternalDelta::active_camera);
      scene.trigger_change_but_not_apply(delta);
    })
  }

  fn set_background(&self, background: Option<SceneBackGround>) {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      s.background = background.clone();
      let delta = background
        .map(MaybeDelta::All)
        .wrap(SceneInternalDelta::background);
      scene.trigger_change_but_not_apply(delta);
    })
  }

  fn update_ext(&self, delta: DeltaOf<DynamicExtension>) {
    self.mutate(|mut scene| unsafe {
      let s = scene.get_mut_ref();
      s.ext.apply(delta.clone()).unwrap();
      scene.trigger_change_but_not_apply(delta.wrap(SceneInternalDelta::ext));
    })
  }
}

#[allow(non_camel_case_types)]
#[derive(Clone)]
pub enum SceneInternalDelta {
  background(DeltaOf<Option<SceneBackGround>>),
  default_camera(DeltaOf<SceneCamera>),
  active_camera(DeltaOf<Option<SceneCamera>>),
  cameras(DeltaOf<Arena<SceneCamera>>),
  lights(DeltaOf<Arena<SceneLight>>),
  models(DeltaOf<Arena<SceneModel>>),
  ext(DeltaOf<DynamicExtension>),
  nodes(DeltaOf<SceneNodeCollection>),
}

impl std::fmt::Debug for SceneInternalDelta {
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

impl IncrementalBase for SceneCoreImpl {
  type Delta = SceneInternalDelta;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    use SceneInternalDelta::*;
    self.nodes.expand(|d| cb(nodes(d)));
    self.background.expand(|d| cb(background(d)));
    self.active_camera.expand(|d| cb(active_camera(d)));
    self.cameras.expand(|d| cb(cameras(d)));
    self.lights.expand(|d| cb(lights(d)));
    self.models.expand(|d| cb(models(d)));
    self.ext.expand(|d| cb(ext(d)));
  }
}

pub type Scene = SharedIncrementalSignal<SceneImpl>;

pub struct SceneImpl {
  pub core: SceneCore,
}

impl SceneImpl {
  pub fn new() -> (Scene, SceneNodeDeriveSystem) {
    let (scene, d) = SceneCoreImpl::new();
    let scene = SceneImpl { core: scene };
    (scene.into_ref(), d)
  }
}

impl IncrementalBase for SceneImpl {
  type Delta = MixSceneDelta;
  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    let core = self.core.read();
    cb(MixSceneDelta::background(
      core.background.clone().map(MaybeDelta::All),
    ));
    cb(MixSceneDelta::active_camera(
      core.active_camera.clone().map(MaybeDelta::All),
    ));
    core.ext.expand(|d| cb(MixSceneDelta::ext(d)));
    core.cameras.iter().for_each(|(_, v)| {
      cb(MixSceneDelta::cameras(
        ContainerRefRetainContentDelta::Insert(v.clone()),
      ))
    });
    core.lights.iter().for_each(|(_, v)| {
      cb(MixSceneDelta::lights(
        ContainerRefRetainContentDelta::Insert(v.clone()),
      ))
    });
    core.models.iter().for_each(|(_, v)| {
      cb(MixSceneDelta::models(
        ContainerRefRetainContentDelta::Insert(v.clone()),
      ))
    });
  }
}

pub trait SceneExt {
  fn root(&self) -> SceneNode;
  fn get_scene_core(&self) -> SceneCore;
  fn create_root_child(&self) -> SceneNode;
  fn compute_full_derived(&self) -> ComputedDerivedTree<SceneNodeDerivedData>;
  fn insert_model(&self, model: SceneModel) -> SceneModelHandle;
  fn remove_model(&self, model: SceneModelHandle);
  fn insert_light(&self, light: SceneLight) -> SceneLightHandle;
  fn remove_light(&self, light: SceneLightHandle);
  fn insert_camera(&self, camera: SceneCamera) -> SceneCameraHandle;
  fn remove_camera(&self, camera: SceneCameraHandle);
  fn set_active_camera(&self, camera: Option<SceneCamera>);
  fn set_background(&self, background: Option<SceneBackGround>);
  fn update_ext(&self, delta: DeltaOf<DynamicExtension>);
}

impl SceneExt for Scene {
  fn root(&self) -> SceneNode {
    self.read().core.read().root().clone()
  }

  fn get_scene_core(&self) -> SceneCore {
    self.read().core.clone()
  }

  fn create_root_child(&self) -> SceneNode {
    self.read().core.create_root_child()
  }

  fn compute_full_derived(&self) -> ComputedDerivedTree<SceneNodeDerivedData> {
    self.read().core.compute_full_derived()
  }

  fn insert_model(&self, model: SceneModel) -> SceneModelHandle {
    self.read().core.insert_model(model)
  }
  fn remove_model(&self, model: SceneModelHandle) {
    self.read().core.remove_model(model)
  }

  fn insert_light(&self, light: SceneLight) -> SceneLightHandle {
    self.read().core.insert_light(light)
  }
  fn remove_light(&self, light: SceneLightHandle) {
    self.read().core.remove_light(light)
  }

  fn insert_camera(&self, camera: SceneCamera) -> SceneCameraHandle {
    self.read().core.insert_camera(camera)
  }
  fn remove_camera(&self, camera: SceneCameraHandle) {
    self.read().core.remove_camera(camera)
  }

  fn set_active_camera(&self, camera: Option<SceneCamera>) {
    self.read().core.set_active_camera(camera);
  }

  fn set_background(&self, background: Option<SceneBackGround>) {
    self.read().core.set_background(background);
  }

  fn update_ext(&self, delta: DeltaOf<DynamicExtension>) {
    self.read().core.update_ext(delta)
  }
}

// /// Manage multi camera view in scene, this idea is not explored but I keep it here
// pub struct CameraGroup {
//   pub cameras: Vec<SceneCamera>,
//   pub current_rendering_camera: usize,
//   /// if no camera provides, we will use default-camera for handling this case easily.
//   pub default_camera: SceneCamera,
// }

#[derive(Clone)]
pub struct SceneNodeCollection {
  pub inner: SceneNodeCollectionImpl,
  pub scene_guid: u64,
}
pub type SceneNodeCollectionImpl =
  Arc<ReactiveTreeCollection<RwLock<TreeCollection<SceneNodeData>>, SceneNodeData>>;

impl SceneNodeCollection {
  pub fn create_node(&self, data: SceneNodeData) -> SceneNode {
    SceneNode::create_new(self.inner.clone(), data, self.scene_guid)
  }
}

impl IncrementalBase for SceneNodeCollection {
  type Delta = TreeMutation<SceneNodeData>;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    let tree = self.inner.inner.read().unwrap();
    tree.expand_with_mapping(|node| node.clone(), |d| cb(d.into()));
  }
}
