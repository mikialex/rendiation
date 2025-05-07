/// https://github.com/raphlinus/crochet
use std::{any::Any, marker::PhantomData, panic::Location};

use fast_hash_collection::FastHashMap;
use rendiation_view_override_model::ViewAutoScalable;

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
  writer: &'a mut SceneWriter,
  memory: &'a mut FunctionMemory<Box<dyn UI3dState>>,
  pub event: Option<UIEventStageCx<'a>>,
  pub dyn_cx: &'a mut DynCx,
  pub current_parent: EntityHandle<SceneNodeEntity>,
  pub current_sub_memory_new_created: bool,
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

struct FunctionMemory<T> {
  states: Vec<T>,
  current_cursor: usize,
  sub_functions: FastHashMap<Location<'static>, Self>,
  sub_functions_next: FastHashMap<Location<'static>, Self>,
}

impl<T> Default for FunctionMemory<T> {
  fn default() -> Self {
    Self {
      states: Default::default(),
      current_cursor: Default::default(),
      sub_functions: Default::default(),
      sub_functions_next: Default::default(),
    }
  }
}

impl<T> FunctionMemory<T> {
  pub fn expect_state_init(&mut self, init: impl FnOnce() -> T) -> &mut T {
    if self.states.len() == self.current_cursor {
      self.states.push(init());
    }
    let r = &mut self.states[self.current_cursor];
    self.current_cursor += 1;
    r
  }

  // return if new created
  #[track_caller]
  pub fn sub_function(&mut self) -> (&mut Self, bool) {
    let location = Location::caller();
    self.current_cursor = 0;
    if let Some(previous_memory) = self.sub_functions.remove(location) {
      (
        self
          .sub_functions_next
          .entry(*location)
          .or_insert(previous_memory),
        false,
      )
    } else {
      (self.sub_functions_next.entry(*location).or_default(), true)
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

impl UI3dCx<'_> {
  pub fn is_mounted(&self) -> bool {
    self.current_sub_memory_new_created
  }

  #[track_caller]
  pub fn scoped<R>(&mut self, f: impl FnOnce(&mut UI3dCx) -> R) -> R {
    let (sub, new_created) = self.memory.sub_function();
    let mut sub_cx = UI3dCx {
      writer: self.writer,
      memory: sub,
      event: self.event,
      dyn_cx: self.dyn_cx,
      current_parent: self.current_parent,
      current_sub_memory_new_created: new_created,
    };
    let r = f(&mut sub_cx);

    let mut drop_cx = UI3dBuildCx {
      writer: self.writer,
    };

    self
      .memory
      .flush(&mut |mut state| state.do_clean_up(&mut drop_cx));
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
          writer: self.writer,
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

// struct StateCell<'a, T> {
//   ptr: *mut T,
//   _marker: PhantomData<fn(&'a ()) -> &'a ()>,
// }

// fn a<'x>(mut x: StateCell<'x, usize>) {
//   struct Cx {}
//   impl Cx {
//     fn next_state(&mut self) -> StateCell<'_, usize> {
//       todo!()
//     }
//   }

//   let mut cx = Cx {};
//   let a = cx.next_state();
//   let mut b = cx.next_state();

//   // drop(cx);

//   // let mut cx2 = Cx {};
//   // let c = cx2.next_state();
//   // // x = c;
//   // b = c;
//   // unsafe { &*b.ptr };
//   // drop(cx2);
//   // b.ptr;
// }

#[track_caller]
pub fn group<R>(
  cx: &mut UI3dCx,
  children: impl FnOnce(&mut UI3dCx, EntityHandle<SceneNodeEntity>) -> R,
) -> GroupResponse<R> {
  cx.scoped(|cx| {
    let (cx, node) = cx.use_node_entity();
    let node = *node;
    let response = children(cx, node);
    GroupResponse { node, response }
  })
}

pub struct GroupResponse<R> {
  pub node: EntityHandle<SceneNodeEntity>,
  pub response: R,
}

pub fn node_entity_ui(cx: &mut UI3dBuildCx) -> EntityHandle<SceneNodeEntity> {
  cx.writer.node_writer.new_entity()
}

impl UI3dCx<'_> {
  pub fn use_node_entity(&mut self) -> (&mut Self, &mut EntityHandle<SceneNodeEntity>) {
    self.use_state_init(node_entity_ui)
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
