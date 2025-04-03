use std::{any::Any, panic::Location};

use fast_hash_collection::FastHashMap;

use crate::*;

pub struct UICx<'a> {
  writer: &'a mut SceneWriter,
  memory: &'a mut FunctionMemory,
  pub event: Option<UIEventStageCx<'a>>,
  pub view_writer: Option<&'a mut SceneWriter>,
  pub current_parent: Option<EntityHandle<SceneNodeEntity>>,
  pub dyn_cx: &'a mut DynCx,
}

#[derive(Default)]
struct FunctionMemory {
  states: Vec<Box<dyn Any>>,
  current_cursor: usize,
  sub_functions: FastHashMap<Location<'static>, Self>,
  sub_functions_next: FastHashMap<Location<'static>, Self>,
}

impl FunctionMemory {
  pub fn expect_state_init<T: Any>(&mut self, init: impl FnOnce() -> T) -> &mut T {
    if self.states.len() == self.current_cursor {
      self.states.push(Box::new(init()));
    }
    let r = self.states[self.current_cursor].downcast_mut().unwrap();
    self.current_cursor += 1;
    r
  }

  pub fn sub_function(&mut self, location: &Location<'static>) -> &mut Self {
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

  pub fn flush(&mut self, drop_state_cb: &mut impl FnMut(Box<dyn Any>)) {
    for (_, mut sub_function) in self.sub_functions.drain() {
      sub_function.cleanup(drop_state_cb);
    }
    std::mem::swap(&mut self.sub_functions, &mut self.sub_functions_next);
  }

  pub fn cleanup(&mut self, mut drop_state_cb: &mut impl FnMut(Box<dyn Any>)) {
    self.states.drain(..).for_each(&mut drop_state_cb);
    self.sub_functions.drain().for_each(|(_, mut f)| {
      f.cleanup(drop_state_cb);
    })
  }
}

#[derive(Copy, Clone)]
pub struct UIEventStageCx<'a> {
  pub platform_event: &'a PlatformEventInput,
  pub interaction_cx: &'a Interaction3dCtx,
}

impl UICx<'_> {
  #[track_caller]
  pub fn scoped<R>(&mut self, f: impl FnOnce(&mut UICx) -> R) -> R {
    let location = Location::caller();
    let mut sub = self.memory.sub_function(location);
    // self.memory = sub;
    // std::mem::swap(&mut sub, &mut self.memory);
    let r = f(self);

    self.memory.flush(&mut |_| {
      // todo
    });
    // std::mem::swap(&mut sub, &mut self.memory);
    // todo fix
    r
  }

  pub fn use_state<'a, T: Any + Default + CxStateDrop<Self>>(&mut self) -> &'a mut T {
    self.use_state_init(|_| T::default())
  }

  pub fn use_state_init<'a, T>(&mut self, init: impl FnOnce(&mut Self) -> T) -> &'a mut T
  where
    T: Any + CxStateDrop<Self>,
  {
    // self.memory.expect_state_init(|| init(self));
    todo!()
  }
}

#[track_caller]
pub fn group(cx: &mut UICx, children: impl FnOnce(&mut UICx, EntityHandle<SceneNodeEntity>)) {
  cx.scoped(|cx| {
    let node = use_node_entity(cx).node;
    children(cx, node);
  });
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
pub struct SceneNodeUIProxy {
  node: EntityHandle<SceneNodeEntity>,
}

pub fn use_node_only_visible<'a>(cx: &'a mut UICx, visible: bool) -> &'a mut SceneNodeUIProxy {
  let node = use_node_entity(cx);

  // todo, set local matrix
  node
}

pub fn use_node<'a>(
  cx: &'a mut UICx,
  local_mat: Mat4<f32>,
  visible: bool,
) -> &'a mut SceneNodeUIProxy {
  let node = use_node_entity(cx);

  // todo, set local matrix and visible
  node
}

pub fn use_node_entity<'a>(cx: &'a mut UICx) -> &'a mut SceneNodeUIProxy {
  cx.use_state_init(|cx| {
    let node = cx.writer.node_writer.new_entity();
    SceneNodeUIProxy { node }
  })
}

impl CxStateDrop<UICx<'_>> for SceneNodeUIProxy {
  fn drop_from_cx(&mut self, cx: &mut UICx) {
    cx.writer.node_writer.delete_entity(self.node);
  }
}

impl CxStateDrop<UICx<'_>> for AttributesMeshEntities {
  fn drop_from_cx(&mut self, cx: &mut UICx<'_>) {
    todo!()
  }
}

pub fn use_mesh_entities<'a, S: Clone + CxStateDrop<UICx> + Into<AttributesMeshData>>(
  cx: &'a mut UICx,
  shape: S,
) -> &'a mut AttributesMeshEntities {
  let previous_create_shape = cx.use_state::<Option<S>>();
  let mesh = cx.use_state_init(|cx| {
    let mesh = shape.clone().into();
    *previous_create_shape = Some(shape);
    cx.writer.write_attribute_mesh(mesh.build())
  });

  // if previous_create_shape != Some(shape) {
  //   todo!()
  // }
  mesh
}

pub fn use_ball_mesh<'a>(cx: &'a mut UICx, radius: f32) -> &'a mut AttributesMeshEntities {
  // use_mesh_entities(cx, radius)
  todo!()
}

pub fn use_model<'a>(
  cx: &'a mut UICx,
  node: SceneNodeUIProxy,
  parent: Option<EntityHandle<SceneNodeEntity>>,
  mesh_entities: &AttributesMeshEntities,
  mouse_interactive: bool,
) -> &'a mut UIWidgetModelProxy {
  cx.use_state_init(|cx| {
    //
    todo!()
  })
}

pub struct UIWidgetModelProxy {
  is_mouse_in: bool,
  is_mouse_down_in_history: bool,

  /// indicate if this widget is interactive to mouse event
  mouse_interactive: bool,

  std_model: EntityHandle<StandardModelEntity>,
  model: EntityHandle<SceneModelEntity>,
  material: EntityHandle<UnlitMaterialEntity>,
}

impl CxStateDrop<UICx<'_>> for UIWidgetModelProxy {
  fn drop_from_cx(&mut self, cx: &mut UICx<'_>) {
    todo!()
  }
}
