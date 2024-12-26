use database::*;
use fast_hash_collection::*;
use rendiation_algebra::*;
use rendiation_device_parallel_compute::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod spd;
use spd::*;

mod filter;
use filter::*;

mod occlusion_test;
use occlusion_test::*;

pub struct TargetWorldBounding {
  pub min: Node<Vec3<f32>>,
  pub max: Node<Vec3<f32>>,
}

pub trait DrawUnitWorldBoundingProvider: ShaderHashProvider {
  fn create_invocation(&self) -> Box<dyn DrawUnitWorldBoundingInvocationProvider>;
  fn bind(&self);
}

pub trait DrawUnitWorldBoundingInvocationProvider {
  fn get_world_bounding(&self) -> TargetWorldBounding;
}

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
  /// mix used view key for different view may cause culling efficiency problem
  ///
  /// the target's depth must be multi sampled.
  ///
  /// todo, support single sampled depth
  pub fn draw(
    &mut self,
    view_key: u32,
    batch: DeviceSceneModelRenderBatch,
    target: RenderPassDescriptorOwned,
    scene_renderer: &impl SceneRenderer,
    camera: EntityHandle<SceneCameraEntity>,
    pass_com: &dyn RenderComponent,
    frame_ctx: &mut FrameCtx,
  ) {
    let last_frame_visibility = self
      .last_frame_visibility
      .entry(view_key)
      .or_insert_with(|| create_gpu_read_write_storage(self.max_scene_model_id, frame_ctx.gpu));

    // first pass
    // draw all visible object in last frame culling result as the occluder
    let last_frame_visible_object = filter_last_frame_visible_object(last_frame_visibility, &batch);
    scene_renderer.make_scene_batch_pass_content(
      SceneModelRenderBatch::Device(last_frame_visible_object),
      camera,
      pass_com,
      frame_ctx,
    );

    // then generate depth pyramid for the occluder
    let (_, depth) = target.depth_stencil_target.clone().unwrap();
    let size = depth.size();

    let depth = depth.expect_standalone_texture_view().0.clone();
    let depth = GPUMultiSample2DDepthTextureView::try_from(depth).unwrap();

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
          format: TextureFormat::Depth32Float,
          view_formats: &[],
          usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        },
        &frame_ctx.gpu.device,
      );
      GPU2DTexture::try_from(tex).unwrap()
    });

    let mut compute_pass = frame_ctx.encoder.begin_compute_pass();

    compute_hierarchy_depth_from_multi_sample_depth_texture(
      &depth,
      pyramid,
      &mut compute_pass,
      &frame_ctx.gpu.device,
    );

    // second pass
    // draw rest object and do occlusion test on all object
    // using depth pyramid. keep culling result for next frame usage
    let pyramid = pyramid.create_default_view();
    let pyramid = GPU2DDepthTextureView::try_from(pyramid).unwrap();
    let rest_objects =
      update_last_frame_visibility_by_all_and_return_objects_that_not_be_occluded_in_this_frame(
        last_frame_visibility,
        &pyramid,
        &batch,
        &mut compute_pass,
        &frame_ctx.gpu.device,
      );

    drop(compute_pass);

    scene_renderer.make_scene_batch_pass_content(
      SceneModelRenderBatch::Device(rest_objects),
      camera,
      pass_com,
      frame_ctx,
    );
  }

  /// if some view key is not used anymore, do cleanup to release underlayer resources
  pub fn cleanup_view_key_culling_states(&mut self, view_key: u32) {
    self.last_frame_visibility.remove(&view_key);
  }
}
