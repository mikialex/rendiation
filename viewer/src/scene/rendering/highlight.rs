// use rendiation_algebra::Vec3;
// use rendiation_webgpu::GPU;

// use crate::*;

// pub struct HighLight {
//   color: Vec3<f32>,
// }

// pub struct HighLighter {
//   source: Attachment<wgpu::TextureFormat>,
// }

// impl PassContent for HighLighter {
//   fn update(
//     &mut self,
//     gpu: &GPU,
//     scene: &mut Scene,
//     resource: &mut ResourcePoolImpl,
//     pass_info: &PassTargetFormatInfo,
//   ) {
//     // get resource pool texture and view , update bindgroup
//     todo!()
//   }

//   fn setup_pass<'a>(
//     &'a self,
//     pass: &mut GPURenderPass<'a>,
//     scene: &'a Scene,
//     pass_info: &'a PassTargetFormatInfo,
//   ) {
//     todo!()
//   }
// }

// pub fn high_light_blend(source: Attachment<wgpu::TextureFormat>) -> impl PassContent {
//   ForwardScene::default()
// }
