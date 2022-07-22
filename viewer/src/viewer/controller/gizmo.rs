use std::{cell::RefCell, rc::Rc};

use interphaser::{
  winit::event::{ElementState, Event, MouseButton, WindowEvent},
  CanvasWindowPositionInfo, Component, WindowState,
};
use rendiation_algebra::{Mat4, Vec3};
use rendiation_geometry::{OptionalNearest, Ray3};
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
  pub fn new(parent: &SceneNode) -> Self {
    let root = parent.create_child();
    let auto_scale = ViewAutoScalable {
      override_position: ViewAutoScalablePositionOverride::SyncNode(root.clone()),
      independent_scale_factor: 100.,
    };
    let auto_scale = Rc::new(RefCell::new(auto_scale));
    Self {
      translate: TranslateGizmo::new(&root, &auto_scale),
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

impl PassContentWithCamera for &mut Gizmo {
  fn render(&mut self, pass: &mut SceneRenderPass, camera: &SceneCamera) {
    let dispatcher = &pass.default_dispatcher();
    self.translate.render(pass, dispatcher, camera)
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
        if self.states.active.only_y() {
          //
        }
      }
    }
  }
  pub fn update(&mut self) {
    let mut ctx = UpdateCtx3D { placeholder: &() };

    self.view.update(&mut self.states, &mut ctx);

    self.root.set_local_matrix(Mat4::translate(1., 0., 1.));
  }
}

fn active(active: impl Fn(&mut AxisActiveState)) -> impl Fn(&mut TranslateGizmoState, &Event3D) {
  move |state, event| {
    active(&mut state.active);
    if let Event3D::MouseDown { world_position } = event {
      println!("{}", world_position);
      state.last_active_world_position = *world_position;
    }
  }
}

impl SceneRenderable for TranslateGizmo {
  fn render(
    &self,
    pass: &mut SceneRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
  ) {
    self.view.render(pass, dispatcher, camera)
  }
}
