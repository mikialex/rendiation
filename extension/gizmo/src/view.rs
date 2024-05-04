use rendiation_mesh_core::MeshBufferHitPoint;
use rendiation_scene_interaction::*;
use webgpu::{FrameRenderPass, RenderComponentAny};
use winit::event::{ElementState, Event, MouseButton};

use crate::*;
use crate::{mouse, mouse_move, CanvasWindowPositionInfo, WindowState};

pub enum ViewReaction<V, T: ApplicableIncremental> {
  /// emit self special event
  ViewEvent(V),
  /// do state mutation
  StateDelta(T::Delta),
}

pub struct EventCtx3D<'a> {
  pub window_states: &'a WindowState,
  pub raw_event: &'a Event<()>,
  pub info: &'a CanvasWindowPositionInfo,
  pub scene: &'a SceneCoreImpl,

  pub event_3d: Option<Event3D>,
  pub node_sys: &'a SceneNodeDeriveSystem,
  pub interactive_ctx: &'a SceneRayInteractiveCtx<'a>,
}

impl<'a> EventCtx3D<'a> {
  pub fn new(
    window_states: &'a WindowState,
    raw_event: &'a Event<()>,
    info: &'a CanvasWindowPositionInfo,
    scene: &'a SceneCoreImpl,
    interactive_ctx: &'a SceneRayInteractiveCtx<'a>,
    node_sys: &'a SceneNodeDeriveSystem,
  ) -> Self {
    Self {
      window_states,
      raw_event,
      info,
      scene,
      event_3d: None,
      interactive_ctx,
      node_sys,
    }
  }
}

pub struct Component3DCollection<T, E> {
  collection: Vec<Box<dyn View3D<T, Event = E>>>,
}

impl<T: ApplicableIncremental, E> Component3DCollection<T, E> {
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

impl<T: ApplicableIncremental, E> ViewBase<T> for Component3DCollection<T, E> {
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
          event.event_3d = map_3d_event(hit, event.raw_event);
          view.event(model, event, cb);
          event.event_3d = None;
        }
        HitReaction::None => view.event(model, event, cb),
      },
    )
  }

  fn update(&mut self, model: &T, delta: &DeltaOf<T>) {
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
    pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  ) {
    for c in &self.collection {
      c.render(pass, dispatcher, camera, scene)
    }
  }
}
