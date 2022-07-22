use interphaser::{
  winit::event::{ElementState, Event, MouseButton, WindowEvent},
  CanvasWindowPositionInfo, Component, WindowState,
};
use rendiation_geometry::{OptionalNearest, Ray3};
use rendiation_renderable_mesh::mesh::{MeshBufferHitPoint, MeshBufferIntersectConfig};

use crate::*;

pub struct System3D;

pub struct EventCtx3D<'a> {
  pub window_states: &'a WindowState,
  pub raw_event: &'a Event<'a, ()>,
  pub info: &'a CanvasWindowPositionInfo,
  pub scene: &'a Scene<WebGPUScene>,

  pub event_3d: Option<Event3D>,
  pub ray: Ray3,
}

impl<'a> EventCtx3D<'a> {
  pub fn new(
    window_states: &'a WindowState,
    raw_event: &'a Event<'a, ()>,
    info: &'a CanvasWindowPositionInfo,
    scene: &'a Scene<WebGPUScene>,
  ) -> Self {
    let normalized_position = info.compute_normalized_position_in_canvas_coordinate(window_states);
    let ray = scene.build_picking_ray_by_view_camera(normalized_position.into());

    Self {
      window_states,
      raw_event,
      info,
      scene,
      event_3d: None,
      ray,
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

fn interact<'a, T>(
  view: T,
  event: &EventCtx3D,
) -> Option<(&'a mut dyn SceneRayInteractive, MeshBufferHitPoint)>
where
  T: IntoIterator<Item = &'a mut dyn SceneRayInteractive>,
{
  let normalized_position = event
    .info
    .compute_normalized_position_in_canvas_coordinate(event.window_states);
  let ray = event
    .scene
    .build_picking_ray_by_view_camera(normalized_position.into());
  interaction_picking_mut(view, ray, &Default::default())
}

pub fn map_3d_events<'a, T>(
  event_ctx: &mut EventCtx3D,
  view: T,
) -> Option<&'a mut dyn SceneRayInteractive>
where
  T: IntoIterator<Item = &'a mut dyn SceneRayInteractive>,
{
  let event = event_ctx.raw_event;
  if let Event::WindowEvent { event, .. } = event {
    match event {
      WindowEvent::CursorMoved { .. } => {
        if let Some((target, details)) = interact(view, event_ctx) {
          event_ctx.event_3d = Event3D::MouseMove {
            world_position: details.hit.position,
          }
          .into();
          return Some(target);
        }
      }
      WindowEvent::MouseInput { state, button, .. } => {
        if let Some((target, details)) = interact(view, event_ctx) {
          if *button == MouseButton::Left {
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
      _ => {}
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

impl<'a, T> SceneRayInteractive for &'a mut dyn Component3D<T> {
  fn ray_pick_nearest(
    &self,
    _world_ray: &Ray3,
    _conf: &MeshBufferIntersectConfig,
  ) -> OptionalNearest<MeshBufferHitPoint> {
    todo!()
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
    for view in &mut self.collection {
      view.event(states, ctx);
    }
    map_3d_events(
      ctx,
      self.collection.iter_mut().map(|c| c.as_mut_interactive()),
    );
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
