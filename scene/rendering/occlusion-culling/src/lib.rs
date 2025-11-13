use rendiation_algebra::*;
use rendiation_device_parallel_compute::*;
use rendiation_fast_down_sampling_2d::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod filter;
use filter::*;

mod occlusion_test;
use occlusion_test::*;

pub struct GPUTwoPassOcclusionCulling {
  /// note, we store the invisible state here, because invisible is zero, which not require special buffer init.
  last_frame_visibility: StorageBufferDataView<[Bool]>,
  depth_pyramid_cache: Option<GPU2DTexture>,
}

impl GPUTwoPassOcclusionCulling {
  /// the `max_scene_model_id` is the maximum **entity id** of scene model could have.
  /// this decides the internal visibility buffer size that addressed by scene model entity id.
  /// user should set this conservatively big enough. if any scene model entity id is larger than
  /// this, the oc will not take effect but the correctness will be ensured
  pub fn new(max_scene_model_id: usize, gpu: &GPU) -> Self {
    let init = ZeroedArrayByArrayLength(max_scene_model_id);
    let last_frame_visibility = create_gpu_read_write_storage(init, gpu);
    Self {
      last_frame_visibility,
      depth_pyramid_cache: Default::default(),
    }
  }
}

impl GPUTwoPassOcclusionCulling {
  /// view key is user defined id for camera related identity
  /// because the per scene model last frame visibility state should be kept for different view
  ///
  /// mix used view key for different view will reduce culling efficiency
  ///
  /// the input batch should be reorderable
  ///
  /// the preflight_content is used to support draw background without initialize another pass.
  /// the return the render pass is used to support subsequent draw without initialize another pass.
  pub fn draw(
    &mut self,
    frame_ctx: &mut FrameCtx,
    batch: &DeviceSceneModelRenderBatch,
    mut target: RenderPassDescription,
    preflight_content: &mut dyn FnMut(ActiveRenderPass) -> ActiveRenderPass,
    scene_renderer: &dyn SceneRenderer,
    camera: &CameraGPU,
    pass_com: &dyn RenderComponent,
    bounding_provider: Box<dyn DrawUnitWorldBoundingProvider>,
    reverse_depth: bool,
  ) -> ActiveRenderPass {
    let pre_culler = batch.stash_culler.clone().unwrap_or(Box::new(NoopCuller));

    let last_frame_invisible = &self.last_frame_visibility;

    // split the batch in to last frame visible and invisible batch
    // todo, this should be optimized
    let last_frame_visible_batch = frame_ctx.access_parallel_compute(|cx| {
      batch
        .clone()
        .with_override_culler(filter_last_frame_visible_object(last_frame_invisible))
        .flush_culler_into_new(cx, true)
    });

    let last_frame_invisible_batch = frame_ctx.access_parallel_compute(|cx| {
      batch
        .clone()
        .with_override_culler(filter_last_frame_visible_object(last_frame_invisible).not())
        .flush_culler_into_new(cx, true)
    });

    // first pass
    // draw all visible object in last frame culling result as the occluder
    let first_pass_batch = last_frame_visible_batch
      .clone()
      .with_override_culler(pre_culler.clone());
    let mut first_pass_batch_draw = scene_renderer.make_scene_batch_pass_content(
      SceneModelRenderBatch::Device(first_pass_batch.clone()),
      camera,
      pass_com,
      frame_ctx,
    );

    let pass = target
      .clone()
      .with_name("occlusion-culling-first-pass")
      .render_ctx(frame_ctx);
    preflight_content(pass).by(&mut first_pass_batch_draw);

    // then generate depth pyramid for the occluder
    let (_, depth) = target.depth_stencil_target.clone().unwrap();
    let size = next_pot_sizer(depth.size());

    let depth = depth.expect_standalone_common_texture_view().clone();

    let required_mip_level_count = MipLevelCount::BySize.get_level_count_wgpu(size);

    if let Some(cache) = &self.depth_pyramid_cache {
      if cache.size() != size.into_gpu_size() || cache.mip_level_count() != required_mip_level_count
      {
        self.depth_pyramid_cache = None;
      }
    }

    let pyramid = self.depth_pyramid_cache.get_or_insert_with(|| {
      let tex = GPUTexture::create(
        TextureDescriptor {
          label: "gpu-occlusion-culling-depth-pyramid".into(),
          size: size.into_gpu_size(),
          mip_level_count: required_mip_level_count,
          sample_count: 1,
          dimension: TextureDimension::D2,
          format: TextureFormat::R32Float, // depth 32 float can not been used in storage texture binding.
          view_formats: &[],
          usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT | TextureUsages::STORAGE_BINDING,
        },
        &frame_ctx.gpu.device,
      );
      GPU2DTexture::try_from(tex).unwrap()
    });

    compute_pot_enlarged_hierarchy_depth(
      depth,
      pyramid,
      frame_ctx,
      &frame_ctx.gpu.device,
      reverse_depth,
    );

    let pyramid = pyramid.create_default_view();
    let pyramid = GPU2DTextureView::try_from(pyramid).unwrap();

    let occlusion_culler = frame_ctx.access_parallel_compute(|cx| {
      test_and_update_last_frame_visibility_for_last_frame_visible_batch_and_return_culler(
        cx,
        &pyramid,
        last_frame_invisible.clone(),
        camera,
        bounding_provider,
        last_frame_visible_batch,
        reverse_depth,
      )
    });

    // second pass, draw rest but not occluded, and update the visibility states
    // todo, check pre_culler if is ok to set before occlusion_culler
    let second_pass_culler = pre_culler.shortcut_or(occlusion_culler);
    let second_pass_batch = last_frame_invisible_batch.with_override_culler(second_pass_culler);

    let mut second_pass_draw = scene_renderer.make_scene_batch_pass_content(
      SceneModelRenderBatch::Device(second_pass_batch),
      camera,
      pass_com,
      frame_ctx,
    );

    // make sure we do not clear what we have drawn in first pass
    target.make_all_channel_and_depth_into_load_op();

    target
      .with_name("occlusion-culling-second-pass")
      .render_ctx(frame_ctx)
      .by(&mut second_pass_draw)
  }
}
