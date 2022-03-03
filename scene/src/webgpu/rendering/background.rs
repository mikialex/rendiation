use rendiation_webgpu::*;

use crate::*;

pub struct BackGroundRendering<'a> {
  scene: &'a mut Scene,
}

// impl<'a> PassContent for BackGroundRendering<'a> {
//   fn render(&mut self, gpu: &GPU, pass: &mut GPURenderPass) {
//     let scene = self.scene;
//     if let Some(camera) = &mut scene.active_camera {
//       scene.background.setup_pass(
//         gpu,
//         pass,
//         scene.active_camera.as_ref().unwrap(),
//         &mut scene.resources,
//       );
//     }
//   }
// }
