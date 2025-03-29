use std::{any::Any, panic::Location};

use crate::*;

pub struct UICx<'a> {
  writer: &'a mut SceneWriter,
  scope: Vec<&'static Location<'static>>,
  memory: Vec<Box<dyn Any>>,
  pub event: Option<UIEventStageCx<'a>>,
  pub view_writer: Option<&'a mut SceneWriter>,
  pub dyn_cx: &'a mut DynCx,
}

pub struct UIEventStageCx<'a> {
  pub platform_event: &'a PlatformEventInput,
  pub interaction_cx: &'a Interaction3dCtx,
}

impl UICx<'_> {
  #[track_caller]
  pub fn scoped<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
    self.scope.push(Location::caller());
    let r = f(self);
    self.scope.pop();
    r
  }

  pub fn use_state<'a, T: Sized>(&mut self) -> &'a mut T {
    todo!()
  }

  pub fn use_state_init<'a, T: Sized>(&mut self, init: impl FnOnce(&mut DynCx) -> T) -> &'a mut T {
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
