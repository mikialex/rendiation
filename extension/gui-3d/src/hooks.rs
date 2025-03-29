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

  pub fn use_state_init<'a, T: Sized, D: FnOnce(&mut T, &mut Self)>(
    &mut self,
    init: impl FnOnce(&mut Self) -> (T, D),
  ) -> &'a mut T {
    // let current_memory =
    todo!()
  }
}

#[track_caller]
pub fn group(cx: &mut UICx, children: impl FnOnce(&mut UICx)) {
  cx.scoped(|cx| {
    let a = cx.use_state::<u32>();
    let b = cx.use_state::<u32>();
    // let node = use_effect(cx, |cx| cx.writer.node_writer.new_entity());
    //
  });
}
