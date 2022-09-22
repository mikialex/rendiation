use interphaser::{
  mouse, mouse_move,
  winit::event::{ElementState, Event, MouseButton},
  CanvasWindowPositionInfo, Component, WindowState,
};

use crate::*;

pub struct System3D;

pub struct EventCtx3D<'a> {
  pub window_states: &'a WindowState,
  pub raw_event: &'a Event<'a, ()>,
  pub info: &'a CanvasWindowPositionInfo,
  pub scene: &'a Scene<WebGPUScene>,

  pub event_3d: Option<Event3D>,
  pub interactive_ctx: &'a SceneRayInteractiveCtx<'a>,
}

impl<'a> EventCtx3D<'a> {
  pub fn new(
    window_states: &'a WindowState,
    raw_event: &'a Event<'a, ()>,
    info: &'a CanvasWindowPositionInfo,
    scene: &'a Scene<WebGPUScene>,
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

pub fn map_3d_events<'a, T, S>(
  event_ctx: &mut EventCtx3D,
  view: T,
  mut on_event: impl FnMut(&mut EventCtx3D, &'a mut dyn Component3D<S>),
) -> Option<&'a mut dyn Component3D<S>>
where
  T: IntoIterator<Item = &'a mut dyn Component3D<S>>,
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

pub struct Component3DCollection<T> {
  collection: Vec<Box<dyn Component3D<T>>>,
}

pub trait Component3D<T>: Component<T, System3D> + SceneRayInteractive + SceneRenderable {
  fn as_mut_interactive(&mut self) -> &mut dyn SceneRayInteractive;
  fn as_interactive(&self) -> &dyn SceneRayInteractive;
}
impl<T, X: Component<T, System3D> + SceneRayInteractive + SceneRenderable> Component3D<T> for X {
  fn as_mut_interactive(&mut self) -> &mut dyn SceneRayInteractive {
    self
  }
  fn as_interactive(&self) -> &dyn SceneRayInteractive {
    self
  }
}

impl<T> Component3DCollection<T> {
  #[must_use]
  pub fn with(mut self, item: impl Component3D<T> + 'static) -> Self {
    self.collection.push(Box::new(item));
    self
  }
}

pub fn collection3d<T>() -> Component3DCollection<T> {
  Component3DCollection {
    collection: Default::default(),
  }
}

impl<T> Component<T, System3D> for Component3DCollection<T> {
  fn event(&mut self, states: &mut T, ctx: &mut EventCtx3D) {
    if let Some(target) = map_3d_events(
      ctx,
      self
        .collection
        .iter_mut() // fixme, how can i pass the compiler here ???!
        .map(|c| unsafe { std::mem::transmute::<_, &mut dyn Component3D<T>>(c.as_mut()) }),
      |ctx, not_hit| {
        not_hit.event(states, ctx);
      },
    ) {
      target.event(states, ctx);
      ctx.event_3d = None;
    }
  }

  fn update(&mut self, states: &T, ctx: &mut UpdateCtx3D) {
    for view in &mut self.collection {
      view.update(states, ctx);
    }
  }
}

impl<T> SceneRenderable for Component3DCollection<T> {
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn DispatcherDyn,
    camera: &SceneCamera,
  ) {
    for c in &self.collection {
      c.render(pass, dispatcher, camera)
    }
  }
}
