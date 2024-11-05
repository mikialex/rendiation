use crate::*;

mod debug_channels;
mod lighting;
mod pipeline;

use debug_channels::*;
use futures::Future;
pub use lighting::*;
pub use pipeline::*;
use reactive::EventSource;
use rendiation_webgpu::*;

pub struct Viewer3dRenderingCtx {
  pub(crate) pipeline: ViewerPipeline,
  rendering_resource: ReactiveQueryJoinUpdater,
  renderer_impl: GLESRenderSystem,
  lighting: LightSystem,
  pool: AttachmentPool,
  gpu: GPU,
  on_encoding_finished: EventSource<ViewRenderedState>,
  expect_read_back_for_next_render_result: bool,
  current_camera_view_projection_inv: Mat4<f32>,
}

impl Viewer3dRenderingCtx {
  pub fn new(gpu: GPU) -> Self {
    let mut renderer_impl = build_default_gles_render_system();
    let mut rendering_resource = ReactiveQueryJoinUpdater::default();
    renderer_impl.register_resource(&mut rendering_resource, &gpu);

    let mut lighting = LightSystem::new(&gpu);
    lighting.register_resource(&mut rendering_resource, &gpu);

    Self {
      rendering_resource,
      renderer_impl,
      pipeline: ViewerPipeline::new(&gpu),
      lighting,
      gpu,
      pool: Default::default(),
      on_encoding_finished: Default::default(),
      expect_read_back_for_next_render_result: false,
      current_camera_view_projection_inv: Default::default(),
    }
  }

  pub fn egui(&mut self, ui: &mut egui::Ui) {
    self.lighting.egui(ui);
    self.pipeline.egui(ui);
  }

  /// only texture could be read. caller must sure the target passed in render call not using
  /// window surface.
  #[allow(unused)] // used in terminal command
  pub fn read_next_render_result(
    &mut self,
  ) -> impl Future<Output = Result<ReadableTextureBuffer, ViewerRenderResultReadBackErr>> {
    self.expect_read_back_for_next_render_result = true;
    use futures::FutureExt;
    self
      .on_encoding_finished
      .once_future(|result| result.clone().read())
      .flatten()
  }

  pub fn resize_view(&mut self) {
    self.pool.clear();
  }

  pub fn update_next_render_camera_info(&mut self, camera_view_projection_inv: Mat4<f32>) {
    self.current_camera_view_projection_inv = camera_view_projection_inv;
  }

  pub fn render(&mut self, target: RenderTargetView, content: &Viewer3dSceneCtx, cx: &mut Context) {
    let mut resource = self.rendering_resource.poll_update_all(cx);
    let renderer = self.renderer_impl.create_impl(&mut resource);

    let render_target = if self.expect_read_back_for_next_render_result
      && matches!(target, RenderTargetView::SurfaceTexture { .. })
    {
      RenderTargetView::Texture(create_empty_2d_texture_view(
        &self.gpu,
        target.size(),
        TextureUsages::all(),
        target.format(),
      ))
    } else {
      target.clone()
    };

    let mut ctx = FrameCtx::new(&self.gpu, target.size(), &self.pool);

    let lighting = self.lighting.create_impl(&mut resource, &mut ctx);

    self.pipeline.render(
      &mut ctx,
      renderer.as_ref(),
      &lighting,
      content,
      &render_target,
      self.current_camera_view_projection_inv,
    );

    // do extra copy to surface texture
    if self.expect_read_back_for_next_render_result
      && matches!(target, RenderTargetView::SurfaceTexture { .. })
    {
      pass("extra final copy to surface")
        .with_color(target, load())
        .render_ctx(&mut ctx)
        .by(&mut rendiation_texture_gpu_process::copy_frame(
          AttachmentView::from_any_view(render_target.clone()),
          None,
        ));
    }
    self.expect_read_back_for_next_render_result = false;
    ctx.final_submit();

    self.on_encoding_finished.emit(&ViewRenderedState {
      target: render_target,
      device: self.gpu.device.clone(),
      queue: self.gpu.queue.clone(),
    });
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
          self.queue.submit_encoder(encoder);
          buffer
        };

        buffer.await.map_err(ViewerRenderResultReadBackErr::Gpu)
      }
      RenderTargetView::SurfaceTexture { .. } => {
        // note: the usage of surface texture could only contains TEXTURE_BINDING, so it's impossible
        // to do any read back from it. the upper layer should be draw content into temp texture for read back
        // and copy back to surface.
        Err(ViewerRenderResultReadBackErr::UnableToReadSurfaceTexture)
      }
    }
  }
}
