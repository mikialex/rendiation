use std::marker::PhantomData;

use incremental::Incremental;
use interphaser::{
  mouse, mouse_move,
  winit::event::{ElementState, Event, MouseButton},
  CanvasWindowPositionInfo, WindowState,
};

use crate::*;

pub struct System3D;

pub struct EventCtx3D<'a> {
  pub window_states: &'a WindowState,
  pub raw_event: &'a Event<'a, ()>,
  pub info: &'a CanvasWindowPositionInfo,
  pub scene: &'a Scene,

  pub event_3d: Option<Event3D>,
  pub interactive_ctx: &'a SceneRayInteractiveCtx<'a>,
}

impl<'a> EventCtx3D<'a> {
  pub fn new(
    window_states: &'a WindowState,
    raw_event: &'a Event<'a, ()>,
    info: &'a CanvasWindowPositionInfo,
    scene: &'a Scene,
    interactive_ctx: &'a SceneRayInteractiveCtx<'a>,
  ) -> Self {
    Self {
      window_states,
      raw_event,
      info,
      scene,
      event_3d: None,
      interactive_ctx,
    }
  }
}

pub struct UpdateCtx3D<'a> {
  pub placeholder: &'a (),
}

impl interphaser::System for System3D {
  type EventCtx<'a> = EventCtx3D<'a>;
  type UpdateCtx<'a> = UpdateCtx3D<'a>;
}

pub fn map_3d_events<'a, T: View<S>, S>(
  event_ctx: &mut EventCtx3D,
  view: T,
  mut on_event: impl FnMut(&mut EventCtx3D, &'a mut dyn View3D<S, Event = T::Event>),
) -> Option<&'a mut dyn View3D<S, Event = T::Event>>
where
  S: Incremental,
  T: IntoIterator<Item = &'a mut dyn View3D<S, Event = T::Event>>,
{
  let event = event_ctx.raw_event;

  if mouse_move(event).is_some() {
    if let Some((target, details)) =
      interaction_picking_mut(view, event_ctx.interactive_ctx, |not_hit| {
        on_event(event_ctx, not_hit)
      })
    {
      event_ctx.event_3d = Event3D::MouseMove {
        world_position: details.hit.position,
      }
      .into();
      return Some(target);
    }
  } else if let Some((button, state)) = mouse(event) {
    if let Some((target, details)) =
      interaction_picking_mut(view, event_ctx.interactive_ctx, |not_hit| {
        on_event(event_ctx, not_hit)
      })
    {
      if button == MouseButton::Left {
        match state {
          ElementState::Pressed => {
            event_ctx.event_3d = Event3D::MouseDown {
              world_position: details.hit.position,
            }
            .into();
          }
          ElementState::Released => {
            event_ctx.event_3d = Event3D::MouseUp {
              world_position: details.hit.position,
            }
            .into();
          }
        }
      }
      return Some(target);
    }
  }

  None
}

pub struct Component3DCollection<T, E> {
  collection: Vec<Box<dyn View3D<T, Event = E>>>,
  // event: PhantomData<E>,
}

pub trait View3D<T: Incremental>: View<T> + SceneRayInteractive + SceneRenderable {
  fn as_mut_interactive(&mut self) -> &mut dyn SceneRayInteractive;
  fn as_interactive(&self) -> &dyn SceneRayInteractive;
}
impl<T: Incremental, X: View<T> + SceneRayInteractive + SceneRenderable> View3D<T> for X {
  fn as_mut_interactive(&mut self) -> &mut dyn SceneRayInteractive {
    self
  }
  fn as_interactive(&self) -> &dyn SceneRayInteractive {
    self
  }
}

impl<T: Incremental, E> Component3DCollection<T, E> {
  #[must_use]
  pub fn with(mut self, item: impl View3D<T, Event = E> + 'static) -> Self {
    self.collection.push(Box::new(item));
    self
  }
}

pub fn collection3d<T, E>() -> Component3DCollection<T, E> {
  Component3DCollection {
    collection: Default::default(),
  }
}

impl<T: Incremental, E> View<T> for Component3DCollection<T, E> {
  type Event = E;

  fn event(
    &mut self,
    model: &mut T,
    event: &mut EventCtx3D,
    cb: &mut dyn FnMut(ViewReaction<Self::Event, T>),
  ) {
    // if let Some(target) = map_3d_events(
    //   event,
    //   self
    //     .collection
    //     .iter_mut() // fixme, how can i pass the compiler here ???!
    //     .map(|c| unsafe { std::mem::transmute::<_, &mut dyn View3D<T>>(c.as_mut()) }),
    //   |ctx, not_hit| {
    //     not_hit.event(model, event, cb);
    //   },
    // ) {
    //   target.event(model, event, cb);
    //   event.event_3d = None;
    // }
  }

  fn update(&mut self, model: &T, delta: &<T as incremental::Incremental>::Delta) {
    todo!()
  }
  // fn event(&mut self, states: &mut T, ctx: &mut EventCtx3D) {
  //   if let Some(target) = map_3d_events(
  //     ctx,
  //     self
  //       .collection
  //       .iter_mut() // fixme, how can i pass the compiler here ???!
  //       .map(|c| unsafe { std::mem::transmute::<_, &mut dyn View3D<T>>(c.as_mut()) }),
  //     |ctx, not_hit| {
  //       not_hit.event(states, ctx);
  //     },
  //   ) {
  //     target.event(states, ctx);
  //     ctx.event_3d = None;
  //   }
  // }

  // fn update(&mut self, states: &T, ctx: &mut UpdateCtx3D) {
  //   for view in &mut self.collection {
  //     view.update(states, ctx);
  //   }
  // }
}

impl<T, E> SceneRenderable for Component3DCollection<T, E> {
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    for c in &self.collection {
      c.render(pass, dispatcher, camera)
    }
  }
}
