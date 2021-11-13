use crate::{ColorAttachment, PassContent, Scene};

use rendiation_algebra::Vec3;

pub struct HighLighter {
  color: Vec3<f32>,
}

impl Default for HighLighter {
  fn default() -> Self {
    Self {
      color: (0., 0.8, 1.).into(),
    }
  }
}

impl HighLighter {
  pub fn draw(&self, mask: ColorAttachment) -> HighLightComposeTask {
    HighLightComposeTask {
      mask,
      lighter: self,
    }
  }
}

pub struct HighLightComposeTask<'a> {
  mask: ColorAttachment,
  lighter: &'a HighLighter,
}

impl<'x> PassContent for HighLightComposeTask<'x> {
  fn update(
    &mut self,
    gpu: &rendiation_webgpu::GPU,
    scene: &mut Scene,
    resource: &mut crate::ResourcePoolImpl,
    pass_info: &rendiation_webgpu::RenderPassInfo,
  ) {
    todo!()
  }

  fn setup_pass<'a>(&'a self, pass: &mut rendiation_webgpu::GPURenderPass<'a>, scene: &'a Scene) {
    todo!()
  }
}

pub struct HighLightDrawMaskTask<T> {
  object: T,
}

pub fn highlight<T>(object: T) -> HighLightDrawMaskTask<T> {
  HighLightDrawMaskTask { object }
}

impl<T> PassContent for HighLightDrawMaskTask<T> {
  fn update(
    &mut self,
    gpu: &rendiation_webgpu::GPU,
    scene: &mut Scene,
    resource: &mut crate::ResourcePoolImpl,
    pass_info: &rendiation_webgpu::RenderPassInfo,
  ) {
    todo!()
  }

  fn setup_pass<'a>(&'a self, pass: &mut rendiation_webgpu::GPURenderPass<'a>, scene: &'a Scene) {
    todo!()
  }
}

// pub struct HighLighter {
//   source: Attachment<wgpu::TextureFormat>,
// }

// impl PassContent for HighLighter {
//   fn update(
//     &mut self,
//     gpu: &GPU,
//     scene: &mut Scene,
//     resource: &mut ResourcePoolImpl,
//     pass_info: &RenderPassInfo,
//   ) {
//     // get resource pool texture and view , update bindgroup
//     todo!()
//   }

//   fn setup_pass<'a>(
//     &'a self,
//     pass: &mut GPURenderPass<'a>,
//     scene: &'a Scene,
//     pass_info: &'a RenderPassInfo,
//   ) {
//     todo!()
//   }
// }

// pub fn high_light_blend(source: Attachment<wgpu::TextureFormat>) -> impl PassContent {
//   ForwardScene::default()
// }
