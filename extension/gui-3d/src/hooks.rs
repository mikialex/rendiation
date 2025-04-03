use std::{any::Any, panic::Location};

use crate::*;

pub struct UICx<'a> {
  writer: &'a mut SceneWriter,
  scope: Vec<&'static Location<'static>>,
  memory: StateMemory,
  memory_visit_stack: Vec<usize>,
  pub event: Option<UIEventStageCx<'a>>,
  pub view_writer: Option<&'a mut SceneWriter>,
  pub current_parent: Option<EntityHandle<SceneNodeEntity>>,
  pub dyn_cx: &'a mut DynCx,
}

pub struct UIEventStageCx<'a> {
  pub platform_event: &'a PlatformEventInput,
  pub interaction_cx: &'a Interaction3dCtx,
}

struct StateCache<T> {
  state: T,
  cleanup: Option<fn(&mut T, &mut UICx)>,
}

struct StateMemory {
  location: &'static Location<'static>,
  memories: Vec<SubState>,
}

impl StateMemory {
  pub fn clean_up(&mut self, cx: &mut UICx) {
    for m in &mut self.memories {
      match m {
        SubState::State(s) => {
          todo!()
          //     let s = s.downcast_mut().unwrap();
          //   if let Some(f) = &s.cleanup {
          //     f(s.downcast_mut().unwrap(), cx)
          //   }
        }
        SubState::SubTree(m) => {
          m.clean_up(cx);
        }
      }
    }
  }
}

enum SubState {
  State(Box<dyn Any>),
  SubTree(StateMemory),
}

impl UICx<'_> {
  fn get_next_memory(&mut self) -> Option<&mut SubState> {
    let mut m = None;
    // for i in 0..self.memory_visit_stack.len() {

    //   m = Some(&mut m.memories[self.memory_visit_stack[i]]);
    // }
    m
  }

  #[track_caller]
  pub fn scoped<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
    self.scope.push(Location::caller());

    let next_memory = self.get_next_memory();

    self.memory_visit_stack.push(0);
    let r = f(self);
    self.scope.pop();
    r
  }

  pub fn use_state<'a, T: Sized>(&mut self) -> &'a mut T {
    todo!()
  }

  pub fn use_state_by<'a, T: Sized>(&mut self, default: T) -> &'a mut T {
    todo!()
  }

  pub fn use_state_init<'a, T>(&mut self, init: impl FnOnce(&mut Self) -> T) -> &'a mut T
  where
    T: Sized + CxStateDrop<Self>,
  {
    // let current_memory =
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

pub fn use_mesh_entities<'a, S: Clone + Into<AttributesMeshData>>(
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
