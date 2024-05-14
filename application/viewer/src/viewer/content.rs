use reactive::{AllocIdx, PollUtils};
use rendiation_algebra::{InnerProductSpace, Mat4, Vec2, Vec3};
use rendiation_controller::{
  ControllerWinitAdapter, InputBound, OrbitController, Transformed3DControllee,
};
use rendiation_geometry::Box3;
use rendiation_mesh_core::MeshBufferIntersectConfig;
use winit::{
  event::{ElementState, Event, KeyEvent, MouseButton},
  keyboard::{KeyCode, PhysicalKey},
};

use crate::*;

pub struct Viewer3dContent {
  pub main_camera: AllocIdx<SceneCameraEntity>,
  pub scene: AllocIdx<SceneEntity>,
  pub pick_config: MeshBufferIntersectConfig,
  pub controller: ControllerWinitAdapter<OrbitController>,
}

impl Viewer3dContent {
  pub fn new() -> Self {
    let controller = OrbitController::default();
    let controller = ControllerWinitAdapter::new(controller);

    Self {
      main_camera: todo!(),
      scene: todo!(),
      controller,
      pick_config: Default::default(),
    }
  }

  pub fn resize_view(&mut self, size: (f32, f32)) {
    // if let Some(camera) = &self.scene.read().core.read().active_camera {
    //   camera.resize(size)
    // }
  }

  // fn fit_camera_view(&self) {
  //   let padding_ratio = 0.1;
  //   let scene_inner = self.scene.read();
  //   let scene = scene_inner.core.read();
  //   let camera = scene.active_camera.clone().unwrap();

  //   // get the bounding box of all selection
  //   let bbox = Box3::empty();
  //   // for model in self.selections.iter_selected() {
  //   //   let handle = model.read().attach_index().unwrap();
  //   //   let handle = scene_inner.core.read().models.get_handle(handle).unwrap();
  //   //   if let Some(b) = self.scene_bounding.get_model_bounding(handle) {
  //   //     bbox.expand_by_other(*b);
  //   //   } else {
  //   //     // for unbound model, we should include the it's coord's center point
  //   //     // todo, add a trait to support logically better center point
  //   //     let world = self.scene_derived.get_world_matrix(&model.read().node);
  //   //     bbox.expand_by_point(world.position());
  //   //   }
  //   // }

  //   if bbox.is_empty() {
  //     println!("not select any thing");
  //     return;
  //   }

  //   let camera = camera.read();

  //   let camera_world = self.scene_derived.get_world_matrix(&camera.node);
  //   let target_center = bbox.center();
  //   let mut object_radius = bbox.min.distance(target_center);

  //   // if we not even have one box
  //   if object_radius == 0. {
  //     object_radius = camera_world.position().distance(target_center);
  //   }

  //   match camera.projection {
  //     CameraProjectionEnum::Perspective(proj) => {
  //       // todo check horizon fov
  //       let half_fov = proj.fov.to_rad() / 2.;
  //       let canvas_half_size = half_fov.tan(); // todo consider near far limit
  //       let padded_canvas_half_size = canvas_half_size * (1.0 - padding_ratio);
  //       let desired_half_fov = padded_canvas_half_size.atan();
  //       let desired_distance = object_radius / desired_half_fov.sin();

  //       let look_at_dir_rev = (camera_world.position() - target_center).normalize();
  //       let desired_camera_center = look_at_dir_rev * desired_distance + target_center;
  //       // we assume camera has no parent!
  //       camera.node.set_local_matrix(Mat4::lookat(
  //         desired_camera_center,
  //         target_center,
  //         Vec3::new(0., 1., 0.),
  //       ))
  //       //
  //     }
  //     _ => {
  //       println!("only perspective camera support fit view for now")
  //     }
  //   }
  // }

  pub fn per_event_update(
    &mut self,
    event: &Event<()>,
    states: &WindowState,
    position_info: CanvasWindowPositionInfo,
  ) {
    // let _bound = InputBound {
    //   origin: (
    //     position_info.absolute_position.x,
    //     position_info.absolute_position.y,
    //   )
    //     .into(),
    //   size: (position_info.size.x, position_info.size.y).into(),
    // };

    // let normalized_screen_position = position_info
    //   .compute_normalized_position_in_canvas_coordinate(states)
    //   .into();

    // // todo, get correct size from render ctx side
    // let camera_view_size =
    //   Size::from_usize_pair_min_one((position_info.size.x as usize, position_info.size.y as
    // usize));

    // enum SelectAction {
    //   DeSelect,
    //   Select(SceneNode),
    //   Nothing,
    // }
    // let mut _act = SelectAction::Nothing;

    // {
    //   let s = self.scene.read();
    //   let scene = &s.core.read();

    //   let interactive_ctx = scene.build_interactive_ctx(
    //     normalized_screen_position,
    //     camera_view_size,
    //     &self.pick_config,
    //     &self.scene_derived,
    //   );

    //   if let Some((MouseButton::Left, ElementState::Pressed)) = mouse(event) {
    //     if let Some((nearest, _)) =
    //       scene.interaction_picking(&interactive_ctx, &mut self.scene_bounding)
    //     {
    //       self.selections.clear();
    //       self.selections.select(nearest);

    //       _act = SelectAction::Select(nearest.read().node.clone());
    //     } else {
    //       _act = SelectAction::DeSelect;
    //     }
    //   };
    // }

    // if let Some((Some(KeyCode::KeyF), ElementState::Pressed)) = keyboard(event) {
    //   self.fit_camera_view();
    // }
  }

  pub fn per_frame_update(&mut self) {
    // struct ControlleeWrapper<'a> {
    //   controllee: &'a SceneNode,
    // }

    // impl<'a> Transformed3DControllee for ControlleeWrapper<'a> {
    //   fn get_matrix(&self) -> Mat4<f32> {
    //     self.controllee.get_local_matrix()
    //   }

    //   fn set_matrix(&mut self, m: Mat4<f32>) {
    //     self.controllee.set_local_matrix(m)
    //   }
    // }

    // let active_camera = self.scene.read().core.read().active_camera.clone();
    // if let Some(camera) = &active_camera {
    //   self.controller.update(&mut ControlleeWrapper {
    //     controllee: &camera.read().node,
    //   });
    // }
  }
}

impl Default for Viewer3dContent {
  fn default() -> Self {
    Self::new()
  }
}

pub struct CanvasWindowPositionInfo {
  /// in window coordinates
  pub absolute_position: Vec2<f32>,
  pub size: Vec2<f32>,
}

impl CanvasWindowPositionInfo {
  pub fn full_window(window_size: (f32, f32)) -> Self {
    Self {
      absolute_position: Vec2::new(0., 0.),
      size: Vec2::new(window_size.0, window_size.1),
    }
  }
}

impl CanvasWindowPositionInfo {
  pub fn compute_normalized_position_in_canvas_coordinate(
    &self,
    states: &WindowState,
  ) -> (f32, f32) {
    let canvas_x = states.mouse_position.0 - self.absolute_position.x;
    let canvas_y = states.mouse_position.1 - self.absolute_position.y;

    (
      canvas_x / self.size.x * 2. - 1.,
      -(canvas_y / self.size.y * 2. - 1.),
    )
  }
}

pub fn window_event(event: &Event<()>) -> Option<&WindowEvent> {
  match event {
    Event::WindowEvent { event, .. } => Some(event),
    _ => None,
  }
}

pub fn mouse(event: &Event<()>) -> Option<(MouseButton, ElementState)> {
  window_event(event).and_then(|e| match e {
    WindowEvent::MouseInput { state, button, .. } => Some((*button, *state)),
    _ => None,
  })
}

pub fn keyboard(event: &Event<()>) -> Option<(Option<KeyCode>, ElementState)> {
  window_event(event).and_then(|e| match e {
    WindowEvent::KeyboardInput {
      event: KeyEvent {
        physical_key,
        state,
        ..
      },
      ..
    } => Some((
      match physical_key {
        PhysicalKey::Code(code) => Some(*code),
        _ => None,
      },
      *state,
    )),
    _ => None,
  })
}

pub fn mouse_move(event: &Event<()>) -> Option<winit::dpi::PhysicalPosition<f64>> {
  window_event(event).and_then(|e| match e {
    WindowEvent::CursorMoved { position, .. } => Some(*position),
    _ => None,
  })
}

pub struct WindowState {
  pub size: (f32, f32),
  pub mouse_position: (f32, f32),
  pub is_left_mouse_down: bool,
  pub is_right_mouse_down: bool,
}

impl WindowState {
  pub fn update_size(&mut self, size: &winit::dpi::PhysicalSize<u32>) {
    self.size.0 = size.width as f32;
    self.size.1 = size.height as f32;
  }

  pub fn mouse_move_to(&mut self, position: &winit::dpi::PhysicalPosition<f64>) {
    self.mouse_position.0 = position.x as f32;
    self.mouse_position.1 = position.y as f32;
  }

  #[allow(clippy::single_match)]
  pub fn event(&mut self, event: &winit::event::Event<()>) {
    match event {
      winit::event::Event::WindowEvent { event, .. } => match event {
        WindowEvent::Resized(size) => {
          self.update_size(size);
        }
        WindowEvent::MouseInput { button, state, .. } => match button {
          MouseButton::Left => match state {
            ElementState::Pressed => self.is_left_mouse_down = true,
            ElementState::Released => self.is_left_mouse_down = false,
          },
          MouseButton::Right => match state {
            ElementState::Pressed => self.is_right_mouse_down = true,
            ElementState::Released => self.is_right_mouse_down = false,
          },
          _ => {}
        },
        WindowEvent::CursorMoved { position, .. } => {
          self.mouse_move_to(position);
        }
        _ => (),
      },
      _ => {}
    }
  }
}

impl Default for WindowState {
  fn default() -> Self {
    Self {
      size: (0.0, 0.0),
      mouse_position: (0.0, 0.0),
      is_left_mouse_down: false,
      is_right_mouse_down: false,
    }
  }
}
