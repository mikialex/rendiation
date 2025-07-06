use fast_hash_collection::*;
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
  max_scene_model_id: usize,
  last_frame_visibility: FastHashMap<u32, StorageBufferDataView<[Bool]>>,
  // todo, improve: we could share the depth pyramid cache for different view
  depth_pyramid_cache: FastHashMap<u32, GPU2DTexture>,
}

impl GPUTwoPassOcclusionCulling {
  /// the `max_scene_model_id` is the maximum **entity id** of scene model could have.
  /// this decides the internal visibility buffer size that addressed by scene model entity id.
  /// user should set this conservatively big enough. if any scene model entity id is larger than
  /// this, the oc will not take effect but the correctness will be ensured
  pub fn new(max_scene_model_count: usize) -> Self {
    Self {
      max_scene_model_id: max_scene_model_count,
      last_frame_visibility: Default::default(),
      depth_pyramid_cache: Default::default(),
    }
  }
}

impl GPUTwoPassOcclusionCulling {
  /// view key is user defined id for viewport/camera related identity
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
    view_key: u32,
    batch: &DeviceSceneModelRenderBatch,
    target: RenderPassDescription,
    preflight_content: &mut dyn FnMut(ActiveRenderPass) -> ActiveRenderPass,
    scene_renderer: &dyn SceneRenderer,
    camera: &CameraGPU,
    pass_com: &dyn RenderComponent,
    bounding_provider: Box<dyn DrawUnitWorldBoundingProvider>,
    reverse_depth: bool,
  ) -> ActiveRenderPass {
    let pre_culler = batch.stash_culler.clone().unwrap_or(Box::new(NoopCuller));

    let last_frame_visibility = self
      .last_frame_visibility
      .entry(view_key)
      .or_insert_with(|| {
        let init = ZeroedArrayByArrayLength(self.max_scene_model_id);
        create_gpu_read_write_storage(init, frame_ctx.gpu)
      });

    // first pass
    // draw all visible object in last frame culling result as the occluder
    let only_last_frame_visible = filter_last_frame_visible_object(last_frame_visibility);
    let first_pass_culler = only_last_frame_visible.shortcut_or(pre_culler.clone());
    let first_pass_batch = batch.clone().with_override_culler(first_pass_culler);

    // must flush culler, because the new culler will update the previous culler's result.
    let first_pass_batch =
      frame_ctx.access_parallel_compute(|cx| first_pass_batch.flush_culler_into_new(cx));

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
    if let Some(cache) = self.depth_pyramid_cache.get(&view_key) {
      if cache.size() != size.into_gpu_size() || cache.mip_level_count() != required_mip_level_count
      {
        self.depth_pyramid_cache.remove(&view_key);
      }
    }

    let pyramid = self.depth_pyramid_cache.entry(view_key).or_insert_with(|| {
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
      test_and_update_last_frame_visibility_use_all_passed_batch_and_return_culler(
        cx,
        &pyramid,
        last_frame_visibility.clone(),
        camera,
        bounding_provider,
        first_pass_batch,
      )
    });

    // second pass, draw rest but not occluded
    let second_pass_culler = only_last_frame_visible
      .not()
      .shortcut_or(pre_culler)
      .shortcut_or(occlusion_culler);
    let second_pass_batch = batch.clone().with_override_culler(second_pass_culler);

    let mut second_pass_draw = scene_renderer.make_scene_batch_pass_content(
      SceneModelRenderBatch::Device(second_pass_batch),
      camera,
      pass_com,
      frame_ctx,
    );

    target
      .with_name("occlusion-culling-second-pass")
      .render_ctx(frame_ctx)
      .by(&mut second_pass_draw)
  }

  /// if some view key is not used anymore, do cleanup to release underlayer resources
  pub fn cleanup_view_key_culling_states(&mut self, view_key: u32) {
    self.last_frame_visibility.remove(&view_key);
  }
}
