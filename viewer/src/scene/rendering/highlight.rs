use rendiation_webgpu::GPU;

use crate::*;

pub struct HighLighter {
  source: Attachment<wgpu::TextureFormat>,
}

impl PassContent for HighLighter {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, resource: &mut ResourcePoolInner) {
    // get resource pool texture and view , update bindgroup
    todo!()
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    scene: &'a Scene,
    resource: &'a ResourcePoolInner,
  ) {
    todo!()
  }
}

pub fn high_light_blend(source: Attachment<wgpu::TextureFormat>) -> impl PassContent {
  ForwardScene::default()
}
