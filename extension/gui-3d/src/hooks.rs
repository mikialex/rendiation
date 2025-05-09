use std::{any::Any, panic::Location};

use fast_hash_collection::FastHashMap;

use crate::*;

pub trait UI3dState: Any + for<'a> CxStateDrop<UI3dBuildCx<'a>> {
  fn as_any_mut(&mut self) -> &mut dyn Any;
  fn do_clean_up(&mut self, cx: &mut UI3dBuildCx);
}
impl<T> UI3dState for T
where
  T: Any + for<'a> CxStateDrop<UI3dBuildCx<'a>>,
{
  fn as_any_mut(&mut self) -> &mut dyn Any {
    self
  }
  fn do_clean_up(&mut self, cx: &mut UI3dBuildCx) {
    self.drop_from_cx(cx);
  }
}

pub struct UI3dBuildCx<'a> {
  writer: &'a mut SceneWriter,
}

pub struct UI3dCx<'a> {
  writer: Option<&'a mut SceneWriter>,
  memory: &'a mut FunctionMemory<Box<dyn UI3dState>>,
  pub event: Option<UIEventStageCx<'a>>,
  pub pick_testing: Option<usize>,
  pub dyn_cx: &'a mut DynCx,
  pub current_parent: Option<EntityHandle<SceneNodeEntity>>,
}

#[derive(Copy, Clone)]
pub struct UIEventStageCx<'a> {
  pub platform_event: &'a PlatformEventInput,
  pub interaction_cx: &'a Interaction3dCtx,
}

pub trait CxStateDrop<T> {
  fn drop_from_cx(&mut self, cx: &mut T);
}

impl<T, X: CxStateDrop<T>> CxStateDrop<T> for Option<X> {
  fn drop_from_cx(&mut self, cx: &mut T) {
    if let Some(x) = self {
      x.drop_from_cx(cx);
    }
  }
}

pub struct FunctionMemory<T> {
  created: bool,
  states: Vec<T>,
  current_cursor: usize,
  sub_functions: FastHashMap<Location<'static>, Self>,
  sub_functions_next: FastHashMap<Location<'static>, Self>,
}

impl<T> Default for FunctionMemory<T> {
  fn default() -> Self {
    Self {
      created: false,
      states: Default::default(),
      current_cursor: Default::default(),
      sub_functions: Default::default(),
      sub_functions_next: Default::default(),
    }
  }
}

impl<T> FunctionMemory<T> {
  pub fn reset_cursor(&mut self) {
    self.current_cursor = 0;
  }
  pub fn expect_state_init(&mut self, init: impl FnOnce() -> T) -> &mut T {
    if self.states.len() == self.current_cursor {
      self.states.push(init());
    }
    let r = &mut self.states[self.current_cursor];
    self.current_cursor += 1;
    r
  }

  #[track_caller]
  pub fn sub_function(&mut self) -> &mut Self {
    let location = Location::caller();
    self.current_cursor = 0;
    if let Some(previous_memory) = self.sub_functions.remove(location) {
      self
        .sub_functions_next
        .entry(*location)
        .or_insert(previous_memory)
    } else {
      self.sub_functions_next.entry(*location).or_default()
    }
  }

  pub fn flush(&mut self, drop_state_cb: &mut impl FnMut(T)) {
    for (_, mut sub_function) in self.sub_functions.drain() {
      sub_function.cleanup(drop_state_cb);
    }
    std::mem::swap(&mut self.sub_functions, &mut self.sub_functions_next);
  }

  pub fn cleanup(&mut self, mut drop_state_cb: &mut impl FnMut(T)) {
    self.states.drain(..).for_each(&mut drop_state_cb);
    self.sub_functions.drain().for_each(|(_, mut f)| {
      f.cleanup(drop_state_cb);
    })
  }
}

impl<'a> UI3dCx<'a> {
  pub fn new_event_stage(
    root_memory: &'a mut FunctionMemory<Box<dyn UI3dState>>,
    event: UIEventStageCx<'a>,
    dyn_cx: &'a mut DynCx,
  ) -> Self {
    Self {
      writer: None,
      memory: root_memory,
      event: Some(event),
      pick_testing: None,
      dyn_cx,
      current_parent: None,
    }
  }

  pub fn new_update_stage(
    root_memory: &'a mut FunctionMemory<Box<dyn UI3dState>>,
    dyn_cx: &'a mut DynCx,
    writer: &'a mut SceneWriter,
  ) -> Self {
    Self {
      writer: Some(writer),
      memory: root_memory,
      event: None,
      pick_testing: None,
      dyn_cx,
      current_parent: None,
    }
  }
}

impl UI3dCx<'_> {
  pub fn view_update(&mut self, updater: impl FnOnce(&mut SceneWriter)) {
    if let Some(w) = &mut self.writer {
      updater(w)
    }
  }
  pub fn view_mounting(&mut self, updater: impl FnOnce(&mut SceneWriter)) {
    let is_new_create = self.is_new_create();
    if let Some(w) = &mut self.writer {
      if is_new_create {
        updater(w)
      }
    }
  }

  pub fn is_new_create(&self) -> bool {
    !self.memory.created
  }

  pub fn execute_as_roo<R>(&mut self, f: impl FnOnce(&mut UI3dCx) -> R) -> R {
    self.memory.reset_cursor();
    let r = f(self);
    self.cleanup_after_execute();
    r
  }

  fn cleanup_after_execute(&mut self) {
    let mut drop_cx = self.writer.as_mut().map(|writer| UI3dBuildCx { writer });

    self.memory.flush(&mut |mut state| {
      if let Some(drop_cx) = &mut drop_cx {
        state.do_clean_up(drop_cx)
      } else {
        panic!("unable to drop")
      }
    });
  }

  #[track_caller]
  pub fn scoped<R>(&mut self, f: impl FnOnce(&mut UI3dCx) -> R) -> R {
    let sub_memory = self.memory.sub_function() as *mut _;

    let self_memory = self.memory as *mut _;

    let r = unsafe {
      self.memory = &mut *sub_memory;
      self.memory.reset_cursor();
      let r = f(self);
      (*sub_memory).created = true;
      self.memory = &mut *self_memory;
      r
    };

    self.cleanup_after_execute();
    r
  }

  pub fn use_state<T>(&mut self) -> (&mut Self, &mut T)
  where
    T: Any + Default + for<'x> CxStateDrop<UI3dBuildCx<'x>>,
  {
    self.use_state_init(|_| T::default())
  }

  pub fn use_state_init<T>(
    &mut self,
    init: impl FnOnce(&mut UI3dBuildCx) -> T,
  ) -> (&mut Self, &mut T)
  where
    T: Any + for<'x> CxStateDrop<UI3dBuildCx<'x>>,
  {
    // this is safe because user can not access previous retrieved state through returned self.
    let s = unsafe { std::mem::transmute_copy(self) };

    let state = self
      .memory
      .expect_state_init(|| {
        let mut cx = UI3dBuildCx {
          writer: self.writer.as_mut().expect("unable to build"),
        };
        Box::new(init(&mut cx))
      })
      .as_mut()
      .as_any_mut()
      .downcast_mut::<T>()
      .unwrap();

    (s, state)
  }
}

#[track_caller]
pub fn group<R>(
  cx: &mut UI3dCx,
  children: impl FnOnce(&mut UI3dCx, EntityHandle<SceneNodeEntity>) -> R,
) -> R {
  cx.scoped(|cx| {
    let (cx, node) = cx.use_node_entity();
    let node = *node;
    let current_parent_backup = cx.current_parent;
    cx.current_parent = Some(node);
    let response = children(cx, node);
    cx.current_parent = current_parent_backup;
    response
  })
}

impl UI3dCx<'_> {
  pub fn use_node_entity(&mut self) -> (&mut Self, &mut EntityHandle<SceneNodeEntity>) {
    self.use_state_init(|cx| cx.writer.node_writer.new_entity())
  }
  pub fn use_mesh_entity(
    &mut self,
    mesh_creator: impl Fn() -> AttributesMeshData,
  ) -> (&mut Self, &mut AttributesMeshEntities) {
    self.use_state_init(|cx| cx.writer.write_attribute_mesh(mesh_creator().build()))
  }

  pub fn use_unlit_material_entity(
    &mut self,
    create: impl Fn() -> UnlitMaterialDataView,
  ) -> (&mut Self, &mut EntityHandle<UnlitMaterialEntity>) {
    self.use_state_init(|cx| create().write(&mut cx.writer.unlit_mat_writer))
  }

  pub fn use_scene_model_entity(
    &mut self,
    create: impl Fn(&mut SceneWriter) -> UIWidgetModelProxy,
  ) -> (&mut Self, &mut UIWidgetModelProxy) {
    self.use_state_init(|cx| create(cx.writer))
  }
}

impl CxStateDrop<UI3dBuildCx<'_>> for EntityHandle<UnlitMaterialEntity> {
  fn drop_from_cx(&mut self, cx: &mut UI3dBuildCx) {
    cx.writer.unlit_mat_writer.delete_entity(*self);
  }
}

impl CxStateDrop<UI3dBuildCx<'_>> for EntityHandle<SceneNodeEntity> {
  fn drop_from_cx(&mut self, cx: &mut UI3dBuildCx) {
    cx.writer.node_writer.delete_entity(*self);
  }
}

impl CxStateDrop<UI3dBuildCx<'_>> for AttributesMeshEntities {
  fn drop_from_cx(&mut self, cx: &mut UI3dBuildCx<'_>) {
    let w = &mut cx.writer;
    self.clean_up(&mut w.mesh_writer, &mut w.buffer_writer);
  }
}

pub struct UIWidgetModelProxy {
  std_model: EntityHandle<StandardModelEntity>,
  model: EntityHandle<SceneModelEntity>,
}

impl UIWidgetModelProxy {
  pub fn new(
    cx: &mut SceneWriter,
    node: &EntityHandle<SceneNodeEntity>,
    material: &EntityHandle<UnlitMaterialEntity>,
    mesh: &AttributesMeshEntities,
  ) -> Self {
    let v = cx;
    let model = StandardModelDataView {
      material: SceneMaterialDataView::UnlitMaterial(*material),
      mesh: mesh.mesh,
      skin: None,
    }
    .write(&mut v.std_model_writer);
    let scene_model = SceneModelDataView {
      model,
      scene: v.scene,
      node: *node,
    }
    .write(&mut v.model_writer);

    Self {
      std_model: model,
      model: scene_model,
    }
  }
  pub fn event(&mut self, cx: &UIEventStageCx) -> Option<UiWidgetModelResponse> {
    todo!()
  }
}

impl CxStateDrop<UI3dBuildCx<'_>> for UIWidgetModelProxy {
  fn drop_from_cx(&mut self, cx: &mut UI3dBuildCx<'_>) {
    cx.writer.std_model_writer.delete_entity(self.std_model);
    cx.writer.model_writer.delete_entity(self.model);
  }
}

pub fn use_view_independent_node(
  cx: &mut UI3dCx,
  node: &EntityHandle<SceneNodeEntity>,
  mat: impl Fn() -> Mat4<f32>,
) {
  let (cx, view_dep) = cx.use_state_init(|_| ViewIndependentModelControl::new(mat()));
  view_dep.update(cx, node);
}

pub struct ViewIndependentModelControl {
  origin_local_mat: Mat4<f32>,
  local_mat_to_sync: Option<Mat4<f32>>,
}

impl ViewIndependentModelControl {
  pub fn new(origin_local_mat: Mat4<f32>) -> Self {
    todo!()
  }

  pub fn update(&mut self, cx: &mut UI3dCx, node: &EntityHandle<SceneNodeEntity>) {
    // if let Some(mat) = self.local_mat_to_sync.take() {
    //   w.set_local_matrix(self.node, mat);
    // }
  }
}
impl CxStateDrop<UI3dBuildCx<'_>> for ViewIndependentModelControl {
  fn drop_from_cx(&mut self, _: &mut UI3dBuildCx<'_>) {}
}
