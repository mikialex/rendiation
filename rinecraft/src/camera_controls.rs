

use rendiation_render_entity::*;
use rendium::WindowEventSession;
use crate::rinecraft::RinecraftState;

pub enum CameraControllerType {
  FPS,
  ORBIT,
}

pub enum CameraController {
  FPS(FPSController),
  ORBIT(OrbitController),
}

impl CameraController {
  pub fn update(&mut self, camera: &mut impl Camera) -> bool {
    match self {
      Self::FPS(controller) => controller.update(camera),
      Self::ORBIT(controller) => controller.update(camera),
    }
  }

  // pub fn init()

  pub fn use_mode(
    camera: & impl Camera,
    controller_type: CameraControllerType,
    event: WindowEventSession<RinecraftState>,
  ) -> Self {
    CameraController::FPS(FPSController::new())
  }
}