use std::hash::Hash;

use crate::*;

pub fn test_and_update_last_frame_visibility_use_all_passed_batch_and_return_culler(
  cx: &mut DeviceParallelComputeCtx,
  depth_pyramid: &GPU2DTextureView,
  last_frame_invisible: StorageBufferDataView<[Bool]>,
  camera: &CameraGPU,
  bounding_provider: Box<dyn DrawUnitWorldBoundingProvider>,
  last_frame_occluder_batch: DeviceSceneModelRenderBatch,
  reverse_depth: bool,
) -> Box<dyn AbstractCullerProvider> {
  let device = cx.gpu.device.clone();

  // the test will update the last_frame visibility when do testing
  let tester = Box::new(OcclusionTester {
    depth_pyramid: depth_pyramid.clone(),
    camera: camera.ubo.clone(),
    bounding_provider,
    last_frame_invisible,
    reverse_depth,
  });

  // update the occluder's visibility for the occluder

  // the occluder culler must be flushed
  assert!(last_frame_occluder_batch.stash_culler.is_none());

  for sub_batch in &last_frame_occluder_batch.sub_batches {
    let scene_models = sub_batch.scene_models.execute_and_expose(cx);
    // update the occluder's visibility for the occluder
    let mut hasher = shader_hasher_from_marker_ty!(OcclusionLastFrameVisibleUpdater);
    tester.hash_pipeline_with_type_info(&mut hasher);

    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut ctx| {
      let scene_models = scene_models.build_shader(&mut ctx);
      let culler = tester.create_invocation(ctx.bindgroups());

      let (id, valid) = scene_models.invocation_logic(ctx.global_invocation_id());
      if_by(valid, || {
        // the result will be write into the visible buffer
        culler.cull(id);
      });

      ctx
    });

    cx.record_pass(|pass, _| {
      let mut binder = BindingBuilder::default();
      scene_models.bind_input(&mut binder);
      tester.bind(&mut binder);
      binder.setup_compute_pass(pass, &device, &pipeline);
    });

    scene_models.dispatch_compute(cx);
  }

  // and return it for the rest
  tester
}

#[derive(Clone)]
struct OcclusionTester {
  depth_pyramid: GPU2DTextureView,
  last_frame_invisible: StorageBufferDataView<[Bool]>,
  camera: UniformBufferDataView<CameraGPUTransform>,
  bounding_provider: Box<dyn DrawUnitWorldBoundingProvider>,
  reverse_depth: bool,
}

impl ShaderHashProvider for OcclusionTester {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.bounding_provider.hash_pipeline(hasher);
    self.reverse_depth.hash(hasher);
  }
}

impl AbstractCullerProvider for OcclusionTester {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn AbstractCullerInvocation> {
    Box::new(OcclusionTesterInvocation {
      depth: cx.bind_by(&self.depth_pyramid),
      camera: cx.bind_by(&self.camera),
      bounding_provider: self.bounding_provider.create_invocation(cx),
      last_frame_invisible: cx.bind_by(&self.last_frame_invisible),
      reverse_depth: self.reverse_depth,
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.depth_pyramid);
    cx.bind(&self.camera);
    self.bounding_provider.bind(cx);
    cx.bind(&self.last_frame_invisible);
  }
}

struct OcclusionTesterInvocation {
  depth: BindingNode<ShaderTexture2D>,
  camera: ShaderReadonlyPtrOf<CameraGPUTransform>,
  bounding_provider: Box<dyn DrawUnitWorldBoundingInvocationProvider>,
  last_frame_invisible: ShaderPtrOf<[Bool]>,
  reverse_depth: bool,
}

impl AbstractCullerInvocation for OcclusionTesterInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool> {
    let target_world_bounding = self.bounding_provider.get_world_bounding(id);
    let is_occluded = self.is_occluded(target_world_bounding);
    self
      .last_frame_invisible
      .index(id)
      .store(is_occluded.into_big_bool());
    is_occluded
  }
}

impl OcclusionTesterInvocation {
  /// return true == occluded
  fn is_occluded(&self, target_world_bounding: TargetWorldBounding) -> Node<bool> {
    let size = hpt_sub_hpt(target_world_bounding.max, target_world_bounding.min);

    let min_xy: Node<Vec2<f32>> = (val(1.), val(1.)).into();
    let max_xy: Node<Vec2<f32>> = (val(0.), val(0.)).into();
    let min_xy = min_xy.make_local_var();
    let max_xy = max_xy.make_local_var();
    let min_z = val(1.).make_local_var();

    let camera_world_position = hpt_uniform_to_hpt(self.camera.world_position().load());
    let render_to_clip = self.camera.view_projection_without_translation().load();

    val(8).into_shader_iter().for_each(|item, _| {
      let min_in_render_space = hpt_sub_hpt(target_world_bounding.min, camera_world_position);

      let corner_x = min_in_render_space.x().make_local_var();
      let corner_y = min_in_render_space.y().make_local_var();
      let corner_z = min_in_render_space.z().make_local_var();

      switch_by(item)
        .case(1, || {
          corner_x.store(corner_x.load() + size.x());
        })
        .case(2, || {
          corner_y.store(corner_y.load() + size.y());
        })
        .case(3, || {
          corner_z.store(corner_z.load() + size.z());
        })
        .case(4, || {
          corner_x.store(corner_x.load() + size.x());
          corner_y.store(corner_y.load() + size.y());
        })
        .case(5, || {
          corner_y.store(corner_y.load() + size.y());
          corner_z.store(corner_z.load() + size.z());
        })
        .case(6, || {
          corner_x.store(corner_x.load() + size.x());
          corner_z.store(corner_z.load() + size.z());
        })
        .case(7, || {
          corner_x.store(corner_x.load() + size.x());
          corner_y.store(corner_y.load() + size.y());
          corner_z.store(corner_z.load() + size.z());
        })
        .end_with_default(|| {});

      let point: Node<Vec4<f32>> =
        (corner_x.load(), corner_y.load(), corner_z.load(), val(1.)).into();
      let clip_position = render_to_clip * point;

      let pos_xyz = clip_position.xyz() / clip_position.w().splat();
      let x = pos_xyz.x().clamp(val(-1.), val(1.0));
      let y = pos_xyz.y().clamp(val(-1.), val(1.0));
      let z = pos_xyz.z().clamp(val(0.), val(1.0));

      let pos_xy: Node<Vec2<f32>> = (x, y).into();
      let pos_xy = pos_xy * val(Vec2::new(0.5, -0.5)) + val(Vec2::new(0.5, 0.5));

      min_xy.store(min_xy.load().min(pos_xy));
      max_xy.store(max_xy.load().max(pos_xy));
      if self.reverse_depth {
        min_z.store(min_z.load().max(z));
      } else {
        min_z.store(min_z.load().min(z));
      }
    });

    let min_xy = min_xy.load();
    let max_xy = max_xy.load();

    let depth_pyramid_size_0 = self.depth.texture_dimension_2d(Some(val(0))).into_f32();

    let box_size = (max_xy - min_xy) * depth_pyramid_size_0;
    let mip_level = box_size.x().max(box_size.y()).log2().ceil().into_u32();
    let mip_level = mip_level.clamp(val(0), self.depth.texture_number_levels() - val(1));

    let depth_pyramid_size = self.depth.texture_dimension_2d(Some(mip_level));
    let limit_x = depth_pyramid_size.x() - val(1);
    let limit_y = depth_pyramid_size.y() - val(1);
    let top_left = (min_xy * depth_pyramid_size.into_f32()).into_u32();

    let l_x = top_left.x().clamp(val(0), limit_x);
    let t_y = top_left.y().clamp(val(0), limit_y);
    let r_x = (l_x + val(1)).clamp(val(0), limit_x);
    let b_y = (t_y + val(1)).clamp(val(0), limit_y);

    let d_0 = self.depth.load_texel((l_x, t_y).into(), mip_level).x();
    let d_1 = self.depth.load_texel((r_x, t_y).into(), mip_level).x();
    let d_2 = self.depth.load_texel((l_x, b_y).into(), mip_level).x();
    let d_3 = self.depth.load_texel((r_x, b_y).into(), mip_level).x();
    if self.reverse_depth {
      let max_depth = d_0.min(d_1).min(d_2).min(d_3);
      min_z.load().less_than(max_depth)
    } else {
      let max_depth = d_0.max(d_1).max(d_2).max(d_3);
      min_z.load().greater_than(max_depth)
    }
  }
}
