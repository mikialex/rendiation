use std::sync::Arc;

use crate::*;

mod contents;
pub use contents::*;

mod pipeline;
use futures::Future;
use pipeline::*;
use reactive::EventSource;
use webgpu::*;

pub struct Viewer3dRenderingCtx {
  pipeline: ViewerPipeline,
  pool: ResourcePool,
  resources: GlobalGPUSystem,
  gpu: Arc<GPU>,
  on_encoding_finished: EventSource<ViewRenderedState>,
}

impl Viewer3dRenderingCtx {
  pub fn new(gpu: Arc<GPU>) -> Self {
    let gpu_resources = GlobalGPUSystem::new(&gpu);
    Self {
      pipeline: ViewerPipeline::new(gpu.as_ref()),
      gpu,
      resources: gpu_resources,
      pool: Default::default(),
      on_encoding_finished: Default::default(),
    }
  }

  /// only texture could be read. caller must sure the target passed in render call not using
  /// surface.
  pub fn read_next_render_result(
    &mut self,
  ) -> impl Future<Output = Result<ReadableTextureBuffer, ViewerRenderResultReadBackErr>> {
    use futures::FutureExt;
    self
      .on_encoding_finished
      .once_future(|result| result.clone().read())
      .flatten()
  }

  pub fn resize_view(&mut self) {
    self.pool.clear();
  }

  pub fn render(&mut self, target: RenderTargetView, content: &mut Viewer3dContent) {
    content.maintain();
    self.resources.maintain();

    let (scene_resource, content_res) = self
      .resources
      .get_or_create_scene_sys_with_content(&content.scene, &content.scene_derived);
    let resource = content_res.read().unwrap();

    let scene = content.scene.read();

    let mut ctx = FrameCtx::new(&self.gpu, target.size(), &self.pool);
    let scene_res = SceneRenderResourceGroup {
      scene: &scene,
      resources: &resource,
      scene_resources: scene_resource,
      node_derives: &content.scene_derived,
    };

    self.pipeline.render(&mut ctx, content, &target, &scene_res);
    ctx.final_submit();

    self.on_encoding_finished.emit(&ViewRenderedState {
      target,
      device: self.gpu.device.clone(),
      queue: self.gpu.queue.clone(),
    })
  }
}

#[derive(Clone)]
struct ViewRenderedState {
  target: RenderTargetView,
  device: GPUDevice,
  queue: GPUQueue,
}

#[derive(Debug)]
pub enum ViewerRenderResultReadBackErr {
  GPU(webgpu::BufferAsyncError),
  UnableToReadSurfaceTexture,
}

impl ViewRenderedState {
  async fn read(self) -> Result<ReadableTextureBuffer, ViewerRenderResultReadBackErr> {
    match self.target {
      RenderTargetView::Texture(tex) => {
        // I have to write this, because I don't know why compiler can't known the encoder is
        // dropped and will not across the await point
        let buffer = {
          let mut encoder = self.device.create_encoder();

          let buffer = encoder.read_texture_2d(
            &self.device,
            &tex.resource.clone().try_into().unwrap(),
            ReadRange {
              size: Size::from_u32_pair_min_one((
                tex.resource.desc.size.width,
                tex.resource.desc.size.height,
              )),
              offset_x: 0,
              offset_y: 0,
            },
          );
          self.queue.submit(Some(encoder.finish()));
          buffer
        };

        buffer.await.map_err(ViewerRenderResultReadBackErr::GPU)
      }
      RenderTargetView::SurfaceTexture { .. } => {
        // note: maybe surface could supported by extra copy, but I'm not sure the surface texture's
        // usage flag.
        Err(ViewerRenderResultReadBackErr::UnableToReadSurfaceTexture)
      }
    }
  }
}
