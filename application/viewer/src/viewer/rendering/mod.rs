use std::sync::Arc;

use crate::*;

mod debug_channels;
mod pipeline;

use debug_channels::*;
use futures::Future;
pub use pipeline::*;
use reactive::EventSource;
use rendiation_webgpu::*;

pub struct Viewer3dRenderingCtx {
  pub(crate) pipeline: ViewerPipeline,
  rendering_resource: ReactiveStateJoinUpdater,
  renderer_impl: GLESRenderSystem,
  pool: AttachmentPool,
  gpu: Arc<GPU>,
  on_encoding_finished: EventSource<ViewRenderedState>,
}

impl Viewer3dRenderingCtx {
  pub fn new(gpu: Arc<GPU>) -> Self {
    let resource_cx = GPUResourceCtx {
      device: gpu.device.clone(),
      queue: gpu.queue.clone(),
    };

    let mut renderer_impl = build_default_gles_render_system(&gpu);
    let mut rendering_resource = ReactiveStateJoinUpdater::default();
    renderer_impl.register_resource(&mut rendering_resource, &resource_cx);
    Self {
      rendering_resource,
      renderer_impl,
      pipeline: ViewerPipeline::new(gpu.as_ref()),
      gpu,
      pool: Default::default(),
      on_encoding_finished: Default::default(),
    }
  }

  /// only texture could be read. caller must sure the target passed in render call not using
  /// window surface.
  #[allow(unused)] // used in terminal command
  pub fn read_next_render_result(
    &self,
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

  pub fn render(
    &mut self,
    target: RenderTargetView,
    content: &Viewer3dSceneCtx,
    cx: &mut std::task::Context,
  ) {
    let mut resource = self.rendering_resource.poll_update_all(cx);
    let renderer = self.renderer_impl.create_impl(&mut resource);

    let mut ctx = FrameCtx::new(&self.gpu, target.size(), &self.pool);

    self
      .pipeline
      .render(&mut ctx, renderer.as_ref(), content, &target);

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
  Gpu(rendiation_webgpu::BufferAsyncError),
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

        buffer.await.map_err(ViewerRenderResultReadBackErr::Gpu)
      }
      RenderTargetView::SurfaceTexture { .. } => {
        // note: maybe surface could supported by extra copy, but I'm not sure the surface texture's
        // usage flag.
        Err(ViewerRenderResultReadBackErr::UnableToReadSurfaceTexture)
      }
    }
  }
}
