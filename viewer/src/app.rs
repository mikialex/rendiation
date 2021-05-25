use winit::event::WindowEvent;

use crate::{
  renderer::Renderer,
  scene::{Scene, SceneResource},
};

pub struct Application {
  scene: Scene,
  resource: SceneResource,
}

impl Application {
  pub fn new() -> Self {
    let scene = Scene::new();
    Self {
      scene,
      resource: SceneResource::new(),
    }
  }

  pub fn render(&mut self, frame: &wgpu::SwapChainFrame, renderer: &mut Renderer) {
    renderer.render(
      &wgpu::RenderPassDescriptor {
        label: "scene pass".into(),
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
          attachment: &frame.output.view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
              r: 0.1,
              g: 0.2,
              b: 0.3,
              a: 1.0,
            }),
            store: true,
          },
        }],
        depth_stencil_attachment: None,
      },
      &mut self.scene,
      &mut self.resource,
    )
  }

  pub fn update(&mut self, event: WindowEvent) {
    //
  }
}
