use database::*;
use fast_hash_collection::*;
use rendiation_algebra::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_webgpu::*;

pub struct GPUTwoPassOcclusionCulling {
  max_scene_model_id: usize,
  last_frame_visibility: FastHashMap<u32, StorageBufferDataView<[Bool]>>,
}

impl GPUTwoPassOcclusionCulling {
  /// the `max_scene_model_id` is the maximum **entity id** of scene model could have.
  /// this decides the internal visibility buffer size that addressed by scene model entity id.
  /// user should set this conservatively big enough. if any scene model entity id is larger than
  /// this, the oc will not take effect but the correctness will be ensured
  pub fn new(max_scene_model_count: usize) -> Self {
    Self {
      max_scene_model_id: max_scene_model_count,
      last_frame_visibility: FastHashMap::default(),
    }
  }
}

impl GPUTwoPassOcclusionCulling {
  /// view key is user defined id for viewport/camera related identity
  /// because the per scene model last frame visibility state should be kept for different view
  ///
  /// mix used view key for different view may cause culling efficiency problem
  pub fn draw(
    &mut self,
    view_key: u32,
    batch: DeviceSceneModelRenderBatch,
    target: RenderPassDescriptorOwned,
    scene_renderer: &impl SceneRenderer,
    camera: EntityHandle<SceneCameraEntity>,
    pass: &dyn RenderComponent,
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
      pass,
      frame_ctx,
    );

    // then generate depth pyramid for the occluder
    let (_, depth) = target.depth_stencil_target.clone().unwrap();
    let depth: GPU2DDepthTextureView = todo!();
    let pyramid = generate_depth_pyramid(&depth);

    // second pass
    // draw rest object and do occlusion on all object
    // using depth pyramid. keep culling result for next frame usage
    let rest_objects =
      update_last_frame_visibility_by_all_and_return_objects_that_not_be_occluded_in_this_frame(
        last_frame_visibility,
        &pyramid,
        &batch,
      );
    scene_renderer.make_scene_batch_pass_content(
      SceneModelRenderBatch::Device(rest_objects),
      camera,
      pass,
      frame_ctx,
    );
  }

  /// if some view key is not used anymore, do cleanup to release underlayer resources
  pub fn cleanup_view_key_culling_states(&mut self, view_key: u32) {
    self.last_frame_visibility.remove(&view_key);
  }
}

fn generate_depth_pyramid(depth: &GPU2DDepthTextureView) -> GPU2DDepthTextureView {
  todo!()
}

fn filter_last_frame_visible_object(
  last_frame: &StorageBufferDataView<[Bool]>,
  batch: &DeviceSceneModelRenderBatch,
) -> DeviceSceneModelRenderBatch {
  todo!()
}

fn update_last_frame_visibility_by_all_and_return_objects_that_not_be_occluded_in_this_frame(
  last_frame: &StorageBufferDataView<[Bool]>,
  depth_pyramid: &GPU2DDepthTextureView,
  batch: &DeviceSceneModelRenderBatch,
) -> DeviceSceneModelRenderBatch {
  todo!()
}
