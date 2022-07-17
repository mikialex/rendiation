use std::{any::Any, cell::RefCell, rc::Rc};

use interphaser::{
  winit::event::{ElementState, Event, MouseButton, WindowEvent},
  CanvasWindowPositionInfo, WindowState,
};
use rendiation_algebra::Vec3;
use rendiation_renderable_mesh::{
  mesh::MeshBufferHitPoint,
  tessellation::{CubeMeshParameter, IndexedMeshTessellator},
};

use crate::*;

pub struct Gizmo {
  // scale: AxisScaleGizmo,
  // rotation: RotationGizmo,
  translate: TranslateGizmo,
}

impl Gizmo {
  pub fn new(root: &SceneNode) -> Self {
    let auto_scale = ViewAutoScalable {
      override_position: None,
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

  pub fn update(&mut self) {}
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

pub struct TranslateGizmo {
  pub enabled: bool,
  states: MovingGizmoState,
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  view: Vec<Box<dyn Component3D<MovingGizmoState>>>,
}

fn build_axis_arrow(
  root: &SceneNode,
  auto_scale: &Rc<RefCell<ViewAutoScalable>>,
) -> Box<dyn SceneRenderableShareable> {
  todo!();
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
struct MovingGizmoState {
  active: AxisActiveState,
  last_active_world_position: Vec3<f32>,
}

impl TranslateGizmo {
  pub fn new(root: &SceneNode, auto_scale: &Rc<RefCell<ViewAutoScalable>>) -> Self {
    let x = build_axis_arrow(root, auto_scale)
      .eventable()
      .update(|s, node| {})
      .on(active(|a| a.x = true));

    let y = build_axis_arrow(root, auto_scale)
      .eventable()
      .on(active(|a| a.y = true));

    let z = build_axis_arrow(root, auto_scale)
      .eventable()
      .on(active(|a| a.z = true));

    let views = vec![x, y, z];

    Self {
      enabled: false,
      states: Default::default(),
      root: root.clone(),
      auto_scale: auto_scale.clone(),
      view: todo!(),
    }
  }

  pub fn event(
    &mut self,
    event: &Event<()>,
    info: &CanvasWindowPositionInfo,
    window_state: &WindowState,
    scene: &Scene<WebGPUScene>,
  ) {
    if !self.enabled {
      return;
    }

    let view = self.view.iter().map(|m| m.as_ref());
    map_3d_events(event, view, &mut self.states, info, window_state, scene);
  }
}

fn active(
  active: impl Fn(&mut AxisActiveState),
) -> impl Fn(&mut MovingGizmoState, &MouseDown3DEvent) {
  move |state, event| {
    active(&mut state.active);
    state.last_active_world_position = event.world_position;
  }
}

impl PassContentWithCamera for &mut TranslateGizmo {
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    if !self.enabled {
      return;
    }

    let dispatcher = &pass.default_dispatcher();

    for c in &self.view {
      c.render(pass, dispatcher, camera)
    }
  }
}

fn interact<S, I: Component3D<S> + Copy, T: IntoIterator<Item = I>>(
  view: T,
  states: &WindowState,
  info: &CanvasWindowPositionInfo,
  scene: &Scene<WebGPUScene>,
) -> Option<(I, MeshBufferHitPoint)> {
  let normalized_position = info.compute_normalized_position_in_canvas_coordinate(states);
  let ray = scene.build_picking_ray_by_view_camera(normalized_position.into());
  interaction_picking(view, ray, &Default::default())
}

pub fn map_3d_events<S, I: Component3D<S> + Copy, T: IntoIterator<Item = I>>(
  event: &Event<()>,
  view: T,
  user_state: &mut S,
  info: &CanvasWindowPositionInfo,
  window_state: &WindowState,
  scene: &Scene<WebGPUScene>,
) {
  if let Event::WindowEvent { event, .. } = event {
    match event {
      WindowEvent::CursorMoved { .. } => {
        if let Some((target, details)) = interact(view, window_state, info, scene) {
          target.event(
            &MouseMove3DEvent {
              world_position: details.hit.position,
            },
            user_state,
          );
        }
      }
      WindowEvent::MouseInput { state, button, .. } => {
        if let Some((target, details)) = interact(view, window_state, info, scene) {
          if *button == MouseButton::Left {
            match state {
              ElementState::Pressed => target.event(
                &MouseDown3DEvent {
                  world_position: details.hit.position,
                },
                user_state,
              ),
              ElementState::Released => todo!(),
            }
          }
        }
      }
      _ => {}
    }
  }
}

pub trait Component3D<T>: SceneRenderable {
  fn event(&self, event: &dyn Any, states: &mut T) {}
  fn update(&mut self, states: &mut T) {}
}

impl<T> Component3D<T> for &dyn Component3D<T> {
  fn event(&self, event: &dyn Any, states: &mut T) {}

  fn update(&mut self, states: &mut T) {}
}
impl<T> SceneRenderable for &dyn Component3D<T> {
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    todo!()
  }
}
