use std::{cell::RefCell, rc::Rc};

use interphaser::{
  winit::event::{ElementState, Event, MouseButton, WindowEvent},
  CanvasWindowPositionInfo, Component, WindowState,
};
use rendiation_algebra::Vec3;
use rendiation_geometry::{Nearest, Ray3};
use rendiation_renderable_mesh::{
  mesh::{MeshBufferHitPoint, MeshBufferIntersectConfig},
  tessellation::{CubeMeshParameter, IndexedMeshTessellator},
};

use crate::{
  helpers::axis::{solid_material, Arrow},
  *,
};

pub struct Gizmo {
  // scale: AxisScaleGizmo,
  // rotation: RotationGizmo,
  translate: TranslateGizmo,
}

impl Gizmo {
  pub fn new(root: &SceneNode) -> Self {
    let auto_scale = ViewAutoScalable {
      override_position: ViewAutoScalablePositionOverride::SyncNode(root.clone()),
      independent_scale_factor: 100.,
    };
    let auto_scale = Rc::new(RefCell::new(auto_scale));
    Self {
      translate: TranslateGizmo::new(root, &auto_scale),
    }
  }

  pub fn event(
    &mut self,
    event: &Event<()>,
    info: &CanvasWindowPositionInfo,
    states: &WindowState,
    scene: &Scene<WebGPUScene>,
  ) {
    self.translate.event(event, info, states, scene)
  }

  pub fn update(&mut self) {
    self.translate.update()
  }
}

// pub struct AxisScaleGizmo {
//   pub root: SceneNode,
//   auto_scale: Rc<RefCell<ViewAutoScalable>>,
//   active: AxisActiveState,
//   x: Box<dyn SceneRenderable>,
//   y: Box<dyn SceneRenderable>,
//   z: Box<dyn SceneRenderable>,
// }

// fn build_box() -> Box<dyn SceneRenderable> {
//   let mesh = CubeMeshParameter::default().tessellate();
//   let mesh = MeshCell::new(MeshSource::new(mesh));
//   todo!();
// }

// pub struct RotationGizmo {
//   pub root: SceneNode,
//   auto_scale: Rc<RefCell<ViewAutoScalable>>,
//   active: AxisActiveState,
//   x: Box<dyn SceneRenderable>,
//   y: Box<dyn SceneRenderable>,
//   z: Box<dyn SceneRenderable>,
// }

// fn build_rotation_circle() -> Box<dyn SceneRenderable> {
//   let mut position = Vec::new();
//   let segments = 50;
//   for i in 0..segments {
//     let p = i as f32 / segments as f32;
//     position.push(Vec3::new(p.cos(), p.sin(), 0.))
//   }
//   todo!();
// }

pub struct System3D;

pub struct EventCtx3D<'a> {
  pub window_states: &'a WindowState,
  pub raw_event: &'a Event<'a, ()>,
  pub info: &'a CanvasWindowPositionInfo,
  pub scene: &'a Scene<WebGPUScene>,

  pub event_3d: Option<Event3D>,
}

pub struct UpdateCtx3D<'a> {
  pub placeholder: &'a (),
}

impl interphaser::System for System3D {
  type EventCtx<'a> = EventCtx3D<'a>;
  type UpdateCtx<'a> = UpdateCtx3D<'a>;
}

pub struct TranslateGizmo {
  pub enabled: bool,
  states: TranslateGizmoState,
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  view: Component3DCollection<TranslateGizmoState>,
}

fn build_axis_arrow(root: &SceneNode, auto_scale: &Rc<RefCell<ViewAutoScalable>>) -> Arrow {
  let (cylinder, tip) = Arrow::default_shape();
  let (cylinder, tip) = (&cylinder, &tip);
  let material = &solid_material((0.8, 0.1, 0.1));
  Arrow::new_reused(root, auto_scale, material, cylinder, tip)
}

#[derive(Copy, Clone, Default)]
pub struct AxisActiveState {
  x: bool,
  y: bool,
  z: bool,
}

impl AxisActiveState {
  pub fn reset(&mut self) {
    *self = Default::default();
  }

  pub fn has_active(&self) -> bool {
    self.x && self.y && self.z
  }
  pub fn only_x(&self) -> bool {
    self.x && !self.y && !self.z
  }
  pub fn only_y(&self) -> bool {
    !self.x && self.y && !self.z
  }
  pub fn only_z(&self) -> bool {
    !self.x && !self.y && self.z
  }
}

#[derive(Default)]
struct TranslateGizmoState {
  active: AxisActiveState,
  last_active_world_position: Vec3<f32>,
}

impl TranslateGizmo {
  pub fn new(root: &SceneNode, auto_scale: &Rc<RefCell<ViewAutoScalable>>) -> Self {
    let x = build_axis_arrow(root, auto_scale)
      .eventable::<TranslateGizmoState>()
      .update(|s, arrow| arrow.root.set_visible(s.active.x))
      .on(active(|a| a.x = true));

    let y = build_axis_arrow(root, auto_scale)
      .eventable::<TranslateGizmoState>()
      .on(active(|a| a.y = true));

    let z = build_axis_arrow(root, auto_scale)
      .eventable::<TranslateGizmoState>()
      .on(active(|a| a.z = true));

    let view = collection3d().with(x).with(y).with(z);

    Self {
      enabled: false,
      states: Default::default(),
      root: root.clone(),
      auto_scale: auto_scale.clone(),
      view,
    }
  }

  pub fn event(
    &mut self,
    event: &Event<()>,
    info: &CanvasWindowPositionInfo,
    window_states: &WindowState,
    scene: &Scene<WebGPUScene>,
  ) {
    if !self.enabled {
      return;
    }

    let mut ctx = EventCtx3D {
      window_states,
      raw_event: event,
      info,
      scene,
      event_3d: None,
    };

    self.view.event(&mut self.states, &mut ctx);

    if let Event::WindowEvent { event, .. } = event {
      if let WindowEvent::CursorMoved { .. } = event {
        if self.states.active.has_active() {
          //
        }
      }
    }
  }
  pub fn update(&mut self) {
    let mut ctx = UpdateCtx3D { placeholder: &() };

    self.view.update(&mut self.states, &mut ctx);
  }
}

fn active(active: impl Fn(&mut AxisActiveState)) -> impl Fn(&mut TranslateGizmoState, &Event3D) {
  move |state, event| {
    active(&mut state.active);
    if let Event3D::MouseDown { world_position } = event {
      state.last_active_world_position = *world_position;
    }
  }
}

impl PassContentWithCamera for &mut TranslateGizmo {
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    if !self.enabled {
      return;
    }

    let dispatcher = &pass.default_dispatcher();
    self.view.render(pass, dispatcher, camera)
  }
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
  ) -> Option<Nearest<MeshBufferHitPoint>> {
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

fn collection3d<T>() -> Component3DCollection<T> {
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
