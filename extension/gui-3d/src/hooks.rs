use std::{
  any::{Any, TypeId},
  panic::Location,
};

use bumpalo::Bump;
use fast_hash_collection::FastHashMap;
pub use rendiation_view_override_model::*;

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
  pub writer: &'a mut SceneWriter,
  pub cx: &'a mut DynCx,
}

pub struct UI3dCx<'a> {
  writer: Option<&'a mut SceneWriter>,
  reader: Option<&'a SceneReader>,
  memory: &'a mut FunctionMemory,
  pub event: Option<UIEventStageCx<'a>>,
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

struct FunctionMemoryState {
  ptr: *mut (),
  type_id: TypeId,
  cleanup_fn: fn(*mut (), *mut ()),
}

#[derive(Default)]
pub struct FunctionMemory {
  created: bool,
  states: Bump,
  states_meta: Vec<FunctionMemoryState>,
  current_cursor: usize,
  sub_functions: FastHashMap<Location<'static>, Self>,
  sub_functions_next: FastHashMap<Location<'static>, Self>,
}

impl FunctionMemory {
  pub fn reset_cursor(&mut self) {
    self.current_cursor = 0;
  }
  pub fn expect_state_init<T: Any, DropCx>(
    &mut self,
    init: impl FnOnce() -> T,
    cleanup: fn(&mut T, &mut DropCx),
  ) -> &mut T {
    unsafe {
      if self.states_meta.len() == self.current_cursor {
        let init = self.states.alloc_with(init);

        let cleanup_fn =
          std::mem::transmute::<fn(&mut T, &mut DropCx), fn(*mut (), *mut ())>(cleanup);

        self.states_meta.push(FunctionMemoryState {
          ptr: init as *mut T as *mut (),
          type_id: TypeId::of::<T>(),
          cleanup_fn,
        });
      }
      let FunctionMemoryState { type_id, ptr, .. } = &mut self.states_meta[self.current_cursor];

      let validate_state_access = true;
      if validate_state_access {
        assert_eq!(*type_id, TypeId::of::<T>());
      }

      self.current_cursor += 1;
      &mut *(*ptr as *mut T)
    }
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

  pub fn flush(&mut self, drop_cx: *mut ()) {
    for (_, mut sub_function) in self.sub_functions.drain() {
      sub_function.cleanup(drop_cx);
    }
    std::mem::swap(&mut self.sub_functions, &mut self.sub_functions_next);
  }

  pub fn cleanup(&mut self, drop_cx: *mut ()) {
    self.states_meta.drain(..).for_each(|meta| {
      (meta.cleanup_fn)(meta.ptr, drop_cx);
    });
    self.sub_functions.drain().for_each(|(_, mut f)| {
      f.cleanup(drop_cx);
    })
  }
}

impl<'a> UI3dCx<'a> {
  pub fn new_event_stage(
    root_memory: &'a mut FunctionMemory,
    event: UIEventStageCx<'a>,
    reader: &'a SceneReader,
    dyn_cx: &'a mut DynCx,
  ) -> Self {
    Self {
      writer: None,
      reader: Some(reader),
      memory: root_memory,
      event: Some(event),
      dyn_cx,
      current_parent: None,
    }
  }

  pub fn new_update_stage(
    root_memory: &'a mut FunctionMemory,
    dyn_cx: &'a mut DynCx,
    writer: &'a mut SceneWriter,
  ) -> Self {
    Self {
      writer: Some(writer),
      reader: None,
      memory: root_memory,
      event: None,
      dyn_cx,
      current_parent: None,
    }
  }
}

impl UI3dCx<'_> {
  /// this updater will also be called when mounting
  pub fn on_update(&mut self, updater: impl FnOnce(&mut SceneWriter, &mut DynCx)) {
    if let Some(w) = &mut self.writer {
      updater(w, self.dyn_cx)
    }
  }
  pub fn on_mounting(
    &mut self,
    updater: impl FnOnce(&mut SceneWriter, &mut DynCx, &Option<EntityHandle<SceneNodeEntity>>),
  ) {
    let is_new_create = self.is_new_create();
    if let Some(w) = &mut self.writer {
      if is_new_create {
        updater(w, self.dyn_cx, &self.current_parent)
      }
    }
  }
  pub fn on_event<R>(
    &mut self,
    updater: impl FnOnce(&UIEventStageCx, &SceneReader, &mut DynCx) -> R,
  ) -> Option<R> {
    let is_new_create = self.is_new_create();
    let mut re = None;
    if let Some(r) = self.reader {
      if let Some(e) = &self.event {
        if is_new_create {
          re = updater(e, r, self.dyn_cx).into();
        }
      }
    }
    re
  }

  pub fn is_new_create(&self) -> bool {
    !self.memory.created
  }

  pub fn execute_as_root<R>(&mut self, f: impl FnOnce(&mut UI3dCx) -> R) -> R {
    self.memory.reset_cursor();
    let r = f(self);
    self.cleanup_after_execute();
    r
  }

  fn cleanup_after_execute(&mut self) {
    if let Some(writer) = &mut self.writer {
      let mut drop_cx = UI3dBuildCx {
        writer,
        cx: self.dyn_cx,
      };
      self
        .memory
        .flush(&mut drop_cx as *mut UI3dBuildCx as *mut ());
    }
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

  pub fn use_plain_state<T>(&mut self) -> (&mut Self, &mut T)
  where
    T: Any + Default,
  {
    self.use_plain_state_init(|_| T::default())
  }

  pub fn use_plain_state_init<T>(
    &mut self,
    init: impl FnOnce(&mut UI3dBuildCx) -> T,
  ) -> (&mut Self, &mut T)
  where
    T: Any,
  {
    #[derive(Default)]
    struct PlainState<T>(T);
    impl<T> CxStateDrop<UI3dBuildCx<'_>> for PlainState<T> {
      fn drop_from_cx(&mut self, _: &mut UI3dBuildCx<'_>) {}
    }

    let (cx, s) = self.use_state_init(|cx| PlainState(init(cx)));
    (cx, &mut s.0)
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

    let state = self.memory.expect_state_init(
      || {
        let mut cx = UI3dBuildCx {
          writer: self.writer.as_mut().expect("unable to build"),
          cx: self.dyn_cx,
        };
        init(&mut cx)
      },
      |state: &mut T, dcx: &mut UI3dBuildCx| unsafe {
        state.do_clean_up(dcx);
        core::ptr::drop_in_place(state);
      },
    );

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

pub fn use_pickable_model(cx: &mut UI3dCx, model: &UIWidgetModelProxy) {
  struct Remove(EntityHandle<SceneModelEntity>);
  impl CxStateDrop<UI3dBuildCx<'_>> for Remove {
    fn drop_from_cx(&mut self, cx: &mut UI3dBuildCx<'_>) {
      access_cx_mut!(
        cx.cx,
        sm_intersection_gp,
        WidgetSceneModelIntersectionGroupConfig
      );
      sm_intersection_gp.group.remove(&self.0);
    }
  }
  cx.use_state_init(|cx| {
    access_cx_mut!(
      cx.cx,
      sm_intersection_gp,
      WidgetSceneModelIntersectionGroupConfig
    );
    sm_intersection_gp.group.insert(model.model);
    Remove(model.model)
  });
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
}

impl CxStateDrop<UI3dBuildCx<'_>> for UIWidgetModelProxy {
  fn drop_from_cx(&mut self, cx: &mut UI3dBuildCx<'_>) {
    cx.writer.std_model_writer.delete_entity(self.std_model);
    cx.writer.model_writer.delete_entity(self.model);
  }
}

pub fn use_state_cx_in_mounting<T, R>(
  cx: &mut UI3dCx,
  init: impl FnOnce(&mut UI3dBuildCx) -> T,
  inner: impl FnOnce(&mut UI3dCx) -> R,
) -> R
where
  T: Any + for<'x> CxStateDrop<UI3dBuildCx<'x>>,
{
  let (cx, state) = cx.use_state_init(init);

  cx.on_mounting(|_, cx, _| unsafe {
    cx.register_cx(state);
  });

  let r = inner(cx);

  cx.on_mounting(|_, cx, _| unsafe {
    cx.unregister_cx::<T>();
  });

  r
}

pub struct ViewIndependentComputer {
  pub override_position: Vec3<f32>,
  pub scale: ViewAutoScalable,
  pub camera_world: Mat4<f32>,
  pub view_height_in_pixel: f32,
  pub camera_proj: PerspectiveProjection<f32>,
}

pub fn use_view_dependent_root<R>(
  cx: &mut UI3dCx,
  node: &EntityHandle<SceneNodeEntity>,
  config: ViewAutoScalable,
  inner: impl Fn(&mut UI3dCx) -> R,
) -> R {
  cx.on_event(|_, _, cx| unsafe {
    access_cx!(cx, access, Box<dyn WidgetEnvAccess>);

    let mut computer = ViewIndependentComputer {
      override_position: access.get_world_mat(*node).unwrap().position(),
      scale: config,
      camera_world: access.get_camera_world_mat(),
      view_height_in_pixel: access.get_view_resolution().y as f32,
      camera_proj: access.get_camera_perspective_proj(),
    };
    cx.register_cx(&mut computer);
  });

  let r = inner(cx);

  cx.on_event(|_, _, cx| unsafe {
    cx.unregister_cx::<ViewIndependentComputer>();
  });

  r
}

pub fn use_view_independent_node(
  cx: &mut UI3dCx,
  node: &EntityHandle<SceneNodeEntity>,
  mat: impl FnOnce() -> Mat4<f32> + 'static,
) {
  let (cx, origin_local_mat) = cx.use_plain_state_init(|_| mat());
  let (cx, local_mat_to_sync) = cx.use_plain_state::<Option<Mat4<f32>>>();

  cx.on_event(|_, reader, cx| {
    access_cx!(cx, config, ViewIndependentComputer);
    access_cx!(cx, world_mat_access, Box<dyn WidgetEnvAccess>);

    let parent_world =
      if let Some(parent_node) = reader.node_reader.read::<SceneNodeParentIdx>(*node) {
        let parent_node = unsafe { EntityHandle::from_raw(parent_node) };
        // todo, now we can only get last frame world matrix, so
        // we can only do view independent stuff in next frame.
        world_mat_access.get_world_mat(parent_node).unwrap()
      } else {
        Mat4::identity()
      };

    let origin_world = parent_world * *origin_local_mat;
    let override_world_mat = config.scale.override_mat(
      origin_world,
      config.override_position,
      config.camera_world,
      config.view_height_in_pixel,
      config.camera_proj,
    );

    *local_mat_to_sync = Some(parent_world.inverse_or_identity() * override_world_mat);
  });

  cx.on_update(|w, _| {
    if let Some(mat) = local_mat_to_sync.take() {
      w.set_local_matrix(*node, mat);
    }
  });
}

pub fn use_interactive_ui_widget_model(
  cx: &mut UI3dCx,
  target: &UIWidgetModelProxy,
) -> Option<UiWidgetModelResponse> {
  let (cx, is_mouse_in) = cx.use_plain_state::<bool>();
  let (cx, is_mouse_down_in_history) = cx.use_plain_state::<bool>();

  cx.on_event(|event, _, _| {
    #[allow(unused_variables)]
    fn debug(label: &str) {
      // println!("{}", label);
    }

    let platform_event = event.platform_event;

    if platform_event.window_state.has_any_mouse_event {
      let mut mouse_entering = false;
      let mut mouse_leave = false;
      let mut mouse_hovering = None;
      let mut mouse_down = None;
      let mut mouse_click = None;

      let is_pressing = platform_event.state_delta.is_left_mouse_pressing();
      let is_releasing = platform_event.state_delta.is_left_mouse_releasing();

      let mut current_frame_hitting = None;
      if let Some((hit, model)) = event.interaction_cx.world_ray_intersected_nearest {
        current_frame_hitting = (model == target.model).then_some(hit);
      }

      if let Some(hitting) = current_frame_hitting {
        if !*is_mouse_in {
          debug("mouse in");
          *is_mouse_in = true;
          mouse_entering = true;
        }
        debug("mouse hovering");
        mouse_hovering = hitting.into();
        if is_pressing {
          debug("mouse down");
          mouse_down = hitting.into();
          *is_mouse_down_in_history = true;
        }
        if is_releasing && *is_mouse_down_in_history {
          debug("click");
          mouse_click = hitting.into();
          *is_mouse_down_in_history = false;
        }
      } else if *is_mouse_in {
        debug("mouse out");
        mouse_leave = true;
        *is_mouse_in = false;
      }

      UiWidgetModelResponse {
        mouse_entering,
        mouse_leave,
        mouse_hovering,
        mouse_down,
        mouse_click,
      }
      .into()
    } else {
      None
    }
  })
  .flatten()
}

pub fn use_inject_cx<T: Default + 'static>(cx: &mut UI3dCx, f: impl FnOnce(&mut UI3dCx)) {
  let (cx, state) = cx.use_plain_state::<T>();
  inject_cx(cx, state, f);
}

pub fn state_pick<T1: 'static, T2: 'static>(
  cx: &mut UI3dCx,
  picker: impl FnOnce(&mut T1) -> &mut T2,
  f: impl FnOnce(&mut UI3dCx),
) {
  let s = cx.dyn_cx.get_cx_ptr::<T1>().unwrap();
  unsafe {
    let picked = picker(&mut *s);
    inject_cx(cx, picked, f)
  }
}

pub fn inject_cx<T: 'static>(cx: &mut UI3dCx, state: &mut T, f: impl FnOnce(&mut UI3dCx)) {
  cx.on_event(|_, _, cx| unsafe {
    cx.register_cx(state);
  });
  cx.on_update(|_, cx| unsafe {
    cx.register_cx(state);
  });

  f(cx);

  cx.on_event(|_, _, cx| unsafe {
    cx.unregister_cx::<T>();
  });
  cx.on_update(|_, cx| unsafe {
    cx.unregister_cx::<T>();
  });
}

pub fn build_attributes_mesh_by(
  mesh_builder: impl FnOnce(&mut AttributesMeshBuilder),
) -> impl FnOnce(&mut UI3dBuildCx) -> AttributesMeshEntities {
  |cx| {
    let mesh = build_attributes_mesh(|builder| mesh_builder(builder));
    cx.writer.write_attribute_mesh(mesh.build())
  }
}
