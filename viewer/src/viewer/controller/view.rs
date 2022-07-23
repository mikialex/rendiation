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

pub fn map_3d_events<'a, T>(
  event_ctx: &mut EventCtx3D,
  view: T,
) -> Option<&'a mut dyn SceneRayInteractive>
where
  T: IntoIterator<Item = &'a mut dyn SceneRayInteractive>,
{
  let event = event_ctx.raw_event;

  if mouse_move(event).is_some() {
    if let Some((target, details)) = interaction_picking_mut(view, event_ctx.interactive_ctx) {
      event_ctx.event_3d = Event3D::MouseMove {
        world_position: details.hit.position,
      }
      .into();
      return Some(target);
    }
  } else if let Some((button, state)) = mouse(event) {
    if let Some((target, details)) = interaction_picking_mut(view, event_ctx.interactive_ctx) {
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
}
impl<T, X: Component<T, System3D> + SceneRayInteractive + SceneRenderable> Component3D<T> for X {
  fn as_mut_interactive(&mut self) -> &mut dyn SceneRayInteractive {
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
    map_3d_events(
      ctx,
      self.collection.iter_mut().map(|c| c.as_mut_interactive()),
    );
    for view in &mut self.collection {
      view.event(states, ctx);
    }
    ctx.event_3d = None;
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
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    for c in &self.collection {
      c.render(pass, dispatcher, camera)
    }
  }
}
