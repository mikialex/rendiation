use std::{cell::RefCell, rc::Rc};

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
  scale: AxisScaleGizmo,
  rotation: RotationGizmo,
  translate: MovingGizmo,
}

impl Gizmo {
  pub fn new(root: &SceneNode) -> Self {
    let auto_scale = ViewAutoScalable {
      override_position: None,
      independent_scale_factor: 100.,
    };
    let auto_scale = Rc::new(RefCell::new(auto_scale));
    todo!()
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
}

pub struct AxisScaleGizmo {
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  active: AxisActiveState,
  x: Box<dyn SceneRenderable>,
  y: Box<dyn SceneRenderable>,
  z: Box<dyn SceneRenderable>,
}

fn build_box() -> Box<dyn SceneRenderable> {
  let mesh = CubeMeshParameter::default().tessellate();
  let mesh = MeshCell::new(MeshSource::new(mesh));
  todo!();
}

pub struct RotationGizmo {
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  active: AxisActiveState,
  x: Box<dyn SceneRenderable>,
  y: Box<dyn SceneRenderable>,
  z: Box<dyn SceneRenderable>,
}

fn build_rotation_circle() -> Box<dyn SceneRenderable> {
  let mut position = Vec::new();
  let segments = 50;
  for i in 0..segments {
    let p = i as f32 / segments as f32;
    position.push(Vec3::new(p.cos(), p.sin(), 0.))
  }
  todo!();
}

pub struct MovingGizmo {
  pub enabled: bool,
  active: AxisActiveState,
  last_active_world_position: Vec3<f32>,
  // x: Box<dyn SceneRenderableShareable>,
  // y: Box<dyn SceneRenderableShareable>,
  // z: Box<dyn SceneRenderableShareable>,
  // xy_hint: Box<dyn SceneRenderableShareable>,
  // xz_hint: Box<dyn SceneRenderableShareable>,
  // zy_hint: Box<dyn SceneRenderableShareable>,
  pub root: SceneNode,
  auto_scale: Rc<RefCell<ViewAutoScalable>>,
  view: Vec<Box<dyn SceneRenderableShareable>>,
}

fn build_axis_arrow(root: &SceneNode) -> Box<dyn SceneRenderableShareable> {
  todo!();
}

#[derive(Copy, Clone)]
pub struct AxisActiveState {
  x: bool,
  y: bool,
  z: bool,
}

impl AxisActiveState {
  pub fn reset(&mut self) {
    self.x = false;
    self.y = false;
    self.z = false;
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

impl MovingGizmo {
  pub fn new(root: &SceneNode) -> Self {
    let x = build_axis_arrow(root).eventable().on(|node, e| {
      if let Some(e) = e.downcast_ref::<MouseDown3DEvent>() {
        //
      }
      //
    });
    let y = build_axis_arrow(root).eventable();
    let z = build_axis_arrow(root).eventable();
    todo!()
  }

  fn interact(
    &mut self,
    states: &WindowState,
    info: &CanvasWindowPositionInfo,
    scene: &Scene<WebGPUScene>,
  ) -> Option<(&mut dyn SceneRenderableShareable, MeshBufferHitPoint)> {
    let normalized_position = info.compute_normalized_position_in_canvas_coordinate(states);
    let ray = scene.build_picking_ray_by_view_camera(normalized_position.into());
    let targets = self.view.iter_mut().map(|m| m.as_mut());
    interaction_picking_mut(targets, ray, &Default::default())
  }

  pub fn event(
    &mut self,
    event: &Event<()>,
    info: &CanvasWindowPositionInfo,
    states: &WindowState,
    scene: &Scene<WebGPUScene>,
  ) {
    if !self.enabled {
      return;
    }

    if let Event::WindowEvent { event, .. } = event {
      match event {
        WindowEvent::KeyboardInput { input, .. } => todo!(),
        WindowEvent::CursorMoved { .. } => {
          if self.active.has_active() {
            if let Some((target, details)) = self.interact(states, info, scene) {
              target.event(&MouseMove3DEvent {
                world_position: details.hit.position,
              });
            }
          }
        }
        WindowEvent::MouseInput { state, button, .. } => {
          if let Some((target, details)) = self.interact(states, info, scene) {
            if *button == MouseButton::Left {
              match state {
                ElementState::Pressed => target.event(&MouseDown3DEvent {
                  world_position: details.hit.position,
                }),
                ElementState::Released => self.active.reset(),
              }
            }
          }
        }
        _ => {}
      }
    }
  }
}

impl PassContentWithCamera for &mut MovingGizmo {
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    if !self.enabled {
      return;
    }

    // if self.active.x {
    //   self.x.render(pass, &pass.default_dispatcher(), camera);
    // }
    // if self.active.y {
    //   self.y.render(pass, &pass.default_dispatcher(), camera);
    // }
    // if self.active.z {
    //   self.z.render(pass, &pass.default_dispatcher(), camera);
    // }
  }
}
