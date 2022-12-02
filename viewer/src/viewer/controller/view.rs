use incremental::Incremental;
use interphaser::{
  mouse, mouse_move,
  winit::event::{ElementState, Event, MouseButton},
  CanvasWindowPositionInfo, WindowState,
};
use rendiation_renderable_mesh::MeshBufferHitPoint;

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
    interaction_picking_mut(
      self.collection.iter_mut().map(|v| v.as_mut()),
      event.interactive_ctx,
      |view, hit| match hit {
        HitReaction::Nearest(hit) => {
          event.event_3d = map_3d_event(hit, event.raw_event).into();
          view.event(model, event, cb);
          event.event_3d = None;
        }
        HitReaction::None => view.event(model, event, cb),
      },
    )
  }

  fn update(&mut self, model: &T, delta: &<T as incremental::Incremental>::Delta) {
    for view in &mut self.collection {
      view.update(model, delta);
    }
  }
}

pub fn map_3d_event(hit: MeshBufferHitPoint, event: &Event<()>) -> Option<Event3D> {
  if mouse_move(event).is_some() {
    Event3D::MouseMove {
      world_position: hit.hit.position,
    }
    .into()
  } else if let Some((button, state)) = mouse(event) {
    if button == MouseButton::Left {
      let e = match state {
        ElementState::Pressed => Event3D::MouseDown {
          world_position: hit.hit.position,
        },
        ElementState::Released => Event3D::MouseUp {
          world_position: hit.hit.position,
        },
      };
      Some(e)
    } else {
      None
    }
  } else {
    None
  }
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
