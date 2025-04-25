use crate::*;

pub fn cascade_shadow_map_uniform(
  inputs: ShadowMapSystemInputs,
  main_view_render_camera_info: impl Stream<Item = (Mat4<f32>, Mat4<f32>)>, // (proj, world)
  config: MultiLayerTexturePackerConfig,
  gpu_ctx: &GPU,
) -> (
  CascadeShadowMapSystem,
  UniformArrayUpdateContainer<CascadeShadowMapInfo, 8>,
) {
  let source_world = inputs.source_world.into_forker();

  let source_proj = inputs.source_proj.into_forker();

  let source_view_proj = source_world
    .clone()
    .collective_zip(source_proj.clone())
    .collective_map(|(w, p)| p * w.inverse_or_identity())
    .into_boxed();

  let (sys, address) = CascadeShadowMapSystem::new(
    config,
    source_world.into_boxed(),
    source_proj.into_boxed(),
    inputs.size,
    main_view_render_camera_info,
  );

  let base_offset = offset_of!(CascadeShadowMapInfo, base);
  let enabled = inputs
    .enabled
    .collective_map(|v| if v { 1 } else { 0 })
    .into_query_update_uniform_array(
      base_offset + offset_of!(BasicShadowMapInfo, enabled),
      gpu_ctx,
    );

  // let map_info = address.into_query_update_uniform_array(
  //   base_offset + offset_of!(BasicShadowMapInfo, map_info),
  //   gpu_ctx,
  // );

  let bias = inputs
    .bias
    .into_query_update_uniform_array(base_offset + offset_of!(BasicShadowMapInfo, bias), gpu_ctx);

  // let shadow_camera_view_proj = source_view_proj.into_query_update_uniform_array(
  //   base_offset + offset_of!(BasicShadowMapInfo, shadow_camera_full_view_proj),
  //   gpu_ctx,
  // );

  let uniforms = UniformBufferDataView::create_default(&gpu_ctx.device);
  let uniforms = UniformArrayUpdateContainer::<CascadeShadowMapInfo, 8>::new(uniforms)
    .with_source(enabled)
    // .with_source(map_info)
    // .with_source(shadow_camera_view_proj)
    .with_source(bias);

  (sys, uniforms)
}

pub struct CascadeShadowMapSystem {
  shadow_map_atlas: Option<GPUTexture>,
  packing: BoxedDynReactiveQuery<u32, [ShadowMapAddressInfo; CASCADE_SHADOW_SPLIT_COUNT]>,
  atlas_resize: Box<dyn Stream<Item = SizeWithDepth> + Unpin>,
  current_size: Option<SizeWithDepth>,
  source_world: BoxedDynReactiveQuery<u32, Mat4<f32>>,
  source_proj: BoxedDynReactiveQuery<u32, [Mat4<f32>; CASCADE_SHADOW_SPLIT_COUNT]>,
}

impl CascadeShadowMapSystem {
  pub fn new(
    config: MultiLayerTexturePackerConfig,
    source_world: BoxedDynReactiveQuery<u32, Mat4<f32>>,
    source_proj: BoxedDynReactiveQuery<u32, Mat4<f32>>,
    size: BoxedDynReactiveQuery<u32, Size>,
    main_view_render_camera_info: impl Stream<Item = (Mat4<f32>, Mat4<f32>)>, // (proj, world)
  ) -> (
    Self,
    BoxedDynReactiveQuery<u32, [SingleShadowMapInfo; CASCADE_SHADOW_SPLIT_COUNT]>,
  ) {
    let (packing, atlas_resize) = reactive_pack_2d_to_3d(config, size);
    let packing = packing.collective_map(convert_pack_result).into_forker();

    let sys = Self {
      shadow_map_atlas: None,
      current_size: None,
      packing: todo!(),
      atlas_resize: Box::new(atlas_resize),
      source_world,
      source_proj: todo!(),
    };
    (sys, todo!())
  }

  #[must_use]
  pub fn update_shadow_maps<'a>(
    &mut self,
    cx: &mut Context,
    frame_ctx: &mut FrameCtx,
    // proj, world
    scene_content: &impl Fn(Mat4<f32>, Mat4<f32>, &mut FrameCtx) -> Box<dyn PassContent + 'a>,
    reversed_depth: bool,
  ) -> GPU2DArrayDepthTextureView {
    todo!()
  }
}

const CASCADE_SHADOW_SPLIT_COUNT: usize = 4;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct CascadeShadowMapInfo {
  pub base: BasicShadowMapInfo,
  pub map_info: Shader140Array<SingleShadowMapInfo, CASCADE_SHADOW_SPLIT_COUNT>,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct SingleShadowMapInfo {
  pub shadow_camera_view_proj: Mat4<f32>,
  pub map_info: ShadowMapAddressInfo,
  pub split_distance: f32,
}

/// return per sub frustum light shadow camera projection mat and split distance
pub fn compute_directional_light_cascade_info(
  camera_world: Mat4<f32>,
  camera_projection: Mat4<f32>,
  world_to_light: Mat4<f32>,
) -> [(OrthographicProjection<f32>, f32); CASCADE_SHADOW_SPLIT_COUNT] {
  let (near, far) = camera_projection.get_near_far_assume_orthographic();
  compute_light_cascade_info(camera_world, camera_projection, world_to_light).map(
    |(min, max, split)| {
      let proj = OrthographicProjection {
        left: min.x,
        right: max.x,
        top: max.y,
        bottom: min.y,
        near,
        far,
      };
      (proj, split)
    },
  )
}

/// return per sub frustum min max point and split distance in light space
pub fn compute_light_cascade_info(
  camera_world: Mat4<f32>,
  camera_projection: Mat4<f32>,
  world_to_light: Mat4<f32>,
) -> [(Vec3<f32>, Vec3<f32>, f32); CASCADE_SHADOW_SPLIT_COUNT] {
  let (near, far) = camera_projection.get_near_far_assume_is_common_projection();

  let world_to_clip = camera_projection * camera_world;
  let clip_to_world = world_to_clip.inverse_or_identity();
  let frustum_corners = [
    Vec3::new(-1.0, 1.0, 0.0),
    Vec3::new(1.0, 1.0, 0.0),
    Vec3::new(1.0, -1.0, 0.0),
    Vec3::new(-1.0, -1.0, 0.0),
    Vec3::new(-1.0, 1.0, 1.0),
    Vec3::new(1.0, 1.0, 1.0),
    Vec3::new(1.0, -1.0, 1.0),
    Vec3::new(-1.0, -1.0, 1.0),
  ]
  .map(|v| clip_to_world * v);

  let ratio = ((far * far) / 1_000_000.0).min(1.0);
  let target_cascade_splits: [f32; CASCADE_SHADOW_SPLIT_COUNT] = std::array::from_fn(|i| {
    let p = (i as f32 + 1.0) / (CASCADE_SHADOW_SPLIT_COUNT as f32);
    let log = near.powf(1.0 - p) * far.powf(p);
    let linear = near + p * (far - near);
    linear.lerp(log, ratio)
  });

  let mut idx = 0;
  target_cascade_splits.map(|split_distance| {
    let far_distance = split_distance;
    let near_distance = if idx == 0 {
      near
    } else {
      target_cascade_splits[idx - 1]
    };

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for idx in 0..8 {
      let distance = if idx < 4 {
        // near plane
        near_distance
      } else {
        far_distance
      };

      let ratio = (distance - near) / (far - near);
      let corner_pair = (frustum_corners[idx % 4], frustum_corners[idx % 4 + 4]);
      let corner_position = corner_pair.0.lerp(corner_pair.1, ratio);

      let corner_position_in_light = world_to_light * corner_position;

      min = min.min(corner_position_in_light);
      max = max.max(corner_position_in_light);
    }
    idx += 1;
    (min, max, split_distance)
  })
}

/// compute the current shading point in which sub frustum
#[shader_fn]
pub fn compute_cascade_index(
  fragment_world_position: Node<Vec3<f32>>,
  camera_world_mat: Node<Mat4<f32>>,
  splits: Node<Vec4<f32>>,
) -> Node<u32> {
  let camera_position = camera_world_mat.position();
  let camera_forward_dir = camera_world_mat.forward().normalize();

  let diff = fragment_world_position - camera_position;
  let distance = diff.dot(camera_forward_dir);

  let x = splits.x();
  let y = splits.y();
  let z = splits.z();

  let offset = val(0_u32).make_local_var();

  if_by(distance.less_than(x), || {
    offset.store(val(0_u32));
  })
  .else_if(distance.less_than(y), || {
    offset.store(val(1_u32));
  })
  .else_if(distance.less_than(z), || {
    offset.store(val(2_u32));
  })
  .else_by(|| {
    offset.store(val(3_u32));
  });

  offset.load()
}
