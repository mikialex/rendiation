use database::RawEntityHandle;
use fast_hash_collection::FastHashMap;
use rendiation_texture_packer::{
  TexturePacker, TexturePackerInit, pack_2d_to_2d::pack_impl::etagere_wrap::EtagerePacker,
  pack_2d_to_3d::MultiLayerTexturePackerRaw,
};

use crate::*;

pub const CASCADE_SHADOW_SPLIT_COUNT: usize = 4;

pub struct CascadeShadowMapLightInput {
  pub source_world: Mat4<f64>,
  pub shadow_near_far: (f32, f32),
  pub size: Size,
  pub bias: ShadowBias,
  pub shadow_enabled: bool,
}

type CascadeShadowPackerImpl = MultiLayerTexturePackerRaw<EtagerePacker>;

pub fn generate_cascade_shadow_info(
  cascade_info_access: &dyn Fn(RawEntityHandle) -> Option<CascadeShadowMapLightInput>,
  packer_size: SizeWithDepth,
  view_camera_proj: Mat4<f32>,
  view_camera_world: Mat4<f64>,
  ndc: &dyn NDCSpaceMapper<f32>,
  split_linear_log_blend_ratio: f32,
  light_uniform_array_index_mapping: &LightArrayAllocateResult,
) -> CascadeShadowPreparer {
  let mut packer = CascadeShadowPackerImpl::init_by_config(packer_size);

  let to_opengl_ndc_space = ndc.transform_into_opengl_standard_ndc();
  let view_camera_proj = to_opengl_ndc_space * view_camera_proj;

  let mut scene_cascade_info = FastHashMap::<
    RawEntityHandle,
    Shader140Array<CascadeShadowMapInfo, MAX_SHADOW_COUNT>,
  >::default();
  let mut light_proj_info =
    FastHashMap::<RawEntityHandle, (Mat4<f64>, [Mat4<f32>; CASCADE_SHADOW_SPLIT_COUNT])>::default();
  let mut light_cascade_info = FastHashMap::<RawEntityHandle, CascadeShadowMapInfo>::default();

  for (scene_id, light_id_mapping) in light_uniform_array_index_mapping.iter() {
    let mut scene_array = Shader140Array::<CascadeShadowMapInfo, MAX_SHADOW_COUNT>::default();

    for (light_id, uniform_array_index) in light_id_mapping.iter() {
      let cascade_uniform = if let Some(input) = cascade_info_access(*light_id) {
        if !input.shadow_enabled {
          CascadeShadowMapInfo {
            enabled: Bool::from(false),
            ..Default::default()
          }
        } else {
          let mut sub_proj_info = [Mat4::default(); CASCADE_SHADOW_SPLIT_COUNT];
          let world_to_light = input.source_world.inverse_or_identity();
          let shadow_near_far = input.shadow_near_far;

          let cascades = compute_cascade_split_info(
            view_camera_world,
            view_camera_proj,
            world_to_light,
            split_linear_log_blend_ratio,
            shadow_near_far,
          );

          let light_world_inv = input.source_world.inverse_or_identity();
          let mut cascade_info =
            Vec::<SingleShadowMapInfo>::with_capacity(CASCADE_SHADOW_SPLIT_COUNT);
          let mut splits = [0.; CASCADE_SHADOW_SPLIT_COUNT];

          let mut base_cascade_range = None;
          for (idx, (sub_proj, split)) in cascades.iter().enumerate() {
            if let Ok(pack) = packer.pack(input.size) {
              let proj = sub_proj.compute_projection_mat(ndc);
              let shadow_center_without_translation_to_shadowmap_ndc =
                proj * light_world_inv.remove_position().into_f32();

              let cascade_range = sub_proj.right - sub_proj.left;
              let base_range = base_cascade_range.get_or_insert(cascade_range);
              let cascade_scale = *base_range / cascade_range;

              cascade_info.push(SingleShadowMapInfo {
                map_info: convert_pack_result(pack),
                shadow_center_without_translation_to_shadowmap_ndc,
                cascade_scale,
                ..Default::default()
              });
              splits[idx] = *split;
              sub_proj_info[idx] = proj;
            } else {
              log::warn!("shadow map pack failed");
            }
          }

          let shadow_world_position = into_hpt(input.source_world.position()).into_uniform();

          light_proj_info.insert(*light_id, (input.source_world, sub_proj_info));

          let info = CascadeShadowMapInfo {
            bias: input.bias,
            shadow_world_position,
            map_info: Shader140Array::from_slice_clamp_or_default(&cascade_info),
            splits: splits.into(),
            enabled: true.into(),
            ..Default::default()
          };

          light_cascade_info.insert(*light_id, info);
          info
        }
      } else {
        CascadeShadowMapInfo {
          enabled: Bool::from(false),
          ..Default::default()
        }
      };

      scene_array.set(*uniform_array_index as usize, cascade_uniform);
    }

    scene_cascade_info.insert(*scene_id, scene_array);
  }

  CascadeShadowPreparer {
    scene_cascade_info,
    light_proj_info,
    light_cascade_info,
    map_size: packer_size,
  }
}

pub struct CascadeShadowPreparer {
  // scene entity -> per-scene cascade uniform array
  pub scene_cascade_info:
    FastHashMap<RawEntityHandle, Shader140Array<CascadeShadowMapInfo, MAX_SHADOW_COUNT>>,
  // light entity -> (world_mat, [4 cascade proj])
  pub light_proj_info:
    FastHashMap<RawEntityHandle, (Mat4<f64>, [Mat4<f32>; CASCADE_SHADOW_SPLIT_COUNT])>,
  // light entity -> cascade info (contains atlas pack addresses)
  pub light_cascade_info: FastHashMap<RawEntityHandle, CascadeShadowMapInfo>,
  pub map_size: SizeWithDepth,
}

#[derive(Default)]
pub struct CascadeShadowGPUCache {
  texture: Option<ShadowAtlas>,
  // scene entity -> per-scene uniform buffer
  uniforms: FastHashMap<
    RawEntityHandle,
    UniformBufferDataView<Shader140Array<CascadeShadowMapInfo, MAX_SHADOW_COUNT>>,
  >,
}

pub struct CascadeShadowGPUData {
  pub shadow_map_atlas: GPU2DArrayDepthTextureView,
  // scene entity -> per-scene uniform buffer
  pub uniforms: FastHashMap<
    RawEntityHandle,
    UniformBufferDataView<Shader140Array<CascadeShadowMapInfo, MAX_SHADOW_COUNT>>,
  >,
  pub reversed_depth: bool,
}

impl CascadeShadowPreparer {
  #[must_use]
  pub fn update_shadow_maps(
    self,
    resource_cache: &mut CascadeShadowGPUCache,
    frame_ctx: &mut FrameCtx,
    scene_content: &mut dyn FnMut(&mut FrameCtx, ShadowMapDrawRequest),
    reversed_depth: bool,
  ) -> CascadeShadowGPUData {
    let shadow_map_atlas = &mut resource_cache.texture;
    let shadow_map_atlas = get_or_create_shadow_atlas(
      "cascade-shadow-map-atlas",
      self.map_size,
      shadow_map_atlas,
      frame_ctx.gpu,
    );
    clear_shadow_map(&shadow_map_atlas, frame_ctx, reversed_depth);

    // do shadowmap updates
    for (light_id, cascade) in self.light_cascade_info.iter() {
      if cascade.enabled == Bool::from(false) {
        continue;
      }
      let proj_info = self.light_proj_info.get(light_id).unwrap();
      let shadow_camera_world = proj_info.0;

      for (slice_index, shadow_view) in cascade.map_info.iter().enumerate() {
        let shadow_view = shadow_view.map_info;

        let write_view = shadow_map_atlas
          .get_layer_view(shadow_view.layer_index as u32)
          .clone();

        // todo, consider merge the pass within the same layer
        // custom dispatcher is not required because we only have depth output.
        let pass = pass("cascade-shadow-map").with_depth(
          &RenderTargetView::from_texture_view(write_view),
          load_and_store(),
          load_and_store(),
        );

        let shadow_camera_proj = proj_info.1[slice_index];

        scene_content(
          frame_ctx,
          ShadowMapDrawRequest {
            shadow_camera_proj,
            shadow_camera_world,
            light_id: *light_id,
            map_desc: ShadowPassDesc {
              desc: pass,
              address: shadow_view,
            },
          },
        );
      }
    }

    let mut old_uniforms = std::mem::take(&mut resource_cache.uniforms);

    let uniforms: FastHashMap<
      RawEntityHandle,
      UniformBufferDataView<Shader140Array<CascadeShadowMapInfo, MAX_SHADOW_COUNT>>,
    > = self
      .scene_cascade_info
      .iter()
      .map(|(scene_id, info)| {
        let uniform = if let Some(existing) = old_uniforms.remove(scene_id) {
          existing.write_at(&frame_ctx.gpu.queue, info, 0);
          existing
        } else {
          create_uniform(
            info.clone(),
            &frame_ctx.gpu.device,
            "cascade-shadow-map-uniform",
          )
        };
        (*scene_id, uniform)
      })
      .collect();

    resource_cache.uniforms = uniforms.clone();

    CascadeShadowGPUData {
      shadow_map_atlas: shadow_map_atlas.get_full_view().clone(),
      uniforms,
      reversed_depth,
    }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct CascadeShadowMapInfo {
  pub bias: ShadowBias,
  pub shadow_world_position: HighPrecisionTranslationUniform,
  pub map_info: Shader140Array<SingleShadowMapInfo, CASCADE_SHADOW_SPLIT_COUNT>,
  pub splits: Vec4<f32>,
  pub enabled: Bool,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct SingleShadowMapInfo {
  pub map_info: ShadowMapAddressInfo,
  pub shadow_center_without_translation_to_shadowmap_ndc: Mat4<f32>,
  /// the world space coverage ratio relative to the first cascade,
  /// used to scale the PCF filter size so that it covers the same world space area
  pub cascade_scale: f32,
}

/// return per sub frustum orth proj and split distance in light space
pub fn compute_cascade_split_info(
  camera_world: Mat4<f64>,
  camera_projection: Mat4<f32>,
  world_to_light: Mat4<f64>,
  split_linear_log_blend_ratio: f32,
  (shadow_near, shadow_far): (f32, f32),
) -> [(OrthographicProjection<f32>, f32); CASCADE_SHADOW_SPLIT_COUNT] {
  let (near, far) = camera_projection.get_near_far_assume_is_common_projection();

  let world_to_clip = camera_projection.into_f64() * camera_world.inverse_or_identity();
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

  let target_cascade_splits: [f32; CASCADE_SHADOW_SPLIT_COUNT] = std::array::from_fn(|i| {
    let p = (i as f32 + 1.0) / (CASCADE_SHADOW_SPLIT_COUNT as f32);
    let log = near.powf(1.0 - p) * far.powf(p);
    let linear = near.lerp(far, p);
    linear.lerp(log, split_linear_log_blend_ratio)
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
      let corner_position = corner_pair.0.lerp(corner_pair.1, ratio.into());

      let corner_position_in_light = world_to_light * corner_position;

      min = min.min(corner_position_in_light.into_f32());
      max = max.max(corner_position_in_light.into_f32());
    }
    idx += 1;

    let proj = OrthographicProjection {
      left: min.x,
      right: max.x,
      top: max.y,
      bottom: min.y,
      near: shadow_near,
      far: shadow_far,
    };

    (proj, split_distance)
  })
}

#[derive(Clone)]
pub struct CascadeShadowMapComponent {
  pub shadow_computer: Arc<dyn AbstractShadowComputer>,
  pub info: UniformBufferDataView<Shader140Array<CascadeShadowMapInfo, MAX_SHADOW_COUNT>>,
  pub reversed_depth: bool,
  pub bias_behavior: ShadowBiasBehaviorConfig,
  /// blend the current cascade with the next one to smooth the transition
  pub filter_across_cascades: bool,
}

impl ShaderHashProvider for CascadeShadowMapComponent {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(&self.reversed_depth);
    self.shadow_computer.hash_pipeline(hasher);
    hasher.hash(&self.bias_behavior);
    hasher.hash(&self.filter_across_cascades);
  }
}

impl AbstractShaderBindingSource for CascadeShadowMapComponent {
  type ShaderBindResult = CascadeShadowMapInvocation;
  fn bind_shader(&self, cx: &mut ShaderBindGroupBuilder) -> CascadeShadowMapInvocation {
    CascadeShadowMapInvocation {
      shadow_computer: self.shadow_computer.bind_shader(cx),
      info: cx.bind_by(&self.info),
      bias_behavior: self.bias_behavior,
      filter_across_cascades: self.filter_across_cascades,
      reversed_depth: self.reversed_depth,
    }
  }
}
impl AbstractBindingSource for CascadeShadowMapComponent {
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    self.shadow_computer.bind_pass(ctx);
    ctx.bind(&self.info);
  }
}

pub struct CascadeShadowMapInvocation {
  shadow_computer: Box<dyn AbstractShadowComputerInvocation>,
  info: ShaderReadonlyPtrOf<Shader140Array<CascadeShadowMapInfo, MAX_SHADOW_COUNT>>,
  bias_behavior: ShadowBiasBehaviorConfig,
  filter_across_cascades: bool,
  reversed_depth: bool,
}

pub struct CascadeShadowMapSingleInvocation {
  sys: CascadeShadowMapInvocation,
  index: Node<u32>,
}

impl ShadowOcclusionQuery for CascadeShadowMapSingleInvocation {
  fn query_shadow_occlusion(
    &self,
    render_position: Node<Vec3<f32>>,
    render_normal: Node<Vec3<f32>>,
    screen_position: Node<Vec2<f32>>,
    camera_world_position: Node<HighPrecisionTranslation>,
    camera_world_none_translation_mat: Node<Mat4<f32>>,
  ) -> Node<f32> {
    self.sys.query_shadow_occlusion_by_idx(
      render_position,
      render_normal,
      self.index,
      screen_position,
      camera_world_position,
      camera_world_none_translation_mat,
    )
  }
}

impl CascadeShadowMapInvocation {
  pub fn query_shadow_occlusion_by_idx(
    &self,
    render_position: Node<Vec3<f32>>,
    render_normal: Node<Vec3<f32>>,
    shadow_idx: Node<u32>,
    screen_position: Node<Vec2<f32>>,
    camera_world_position: Node<HighPrecisionTranslation>,
    camera_world_none_translation_mat: Node<Mat4<f32>>,
  ) -> Node<f32> {
    let enabled = self.info.index(shadow_idx).enabled().load();
    enabled.into_bool().select_branched(
      || {
        let shadow_info = self.info.index(shadow_idx);
        let shadow_world_position = shadow_info.shadow_world_position().load();
        let bias = shadow_info.bias().load();

        let split_distances = shadow_info.splits().load();
        let cascade_index = compute_cascade_index(
          render_position,
          camera_world_none_translation_mat,
          split_distances,
        );

        let cascade_info = shadow_info.map_info().index(cascade_index).load().expand();

        let shadow_position = compute_shadow_position(
          render_position,
          render_normal,
          shadow_world_position,
          camera_world_position,
          bias,
          cascade_info.map_info,
          cascade_info.shadow_center_without_translation_to_shadowmap_ndc,
          self.bias_behavior.use_n_dot_l_normal_offset,
          self.reversed_depth,
        );

        if DEBUG_SHADOW_UV {
          let debug = vec3_node((shadow_position.xy(), val(0.)));
          DEFAULT_DISPLAY_DEBUG.with_borrow_mut(|v| {
            if let Some(v) = v {
              v.store(debug);
            }
          });
        }

        let current_occlusion = self.shadow_computer.compute_shadow(
          shadow_position,
          screen_position,
          cascade_info.map_info,
          cascade_info.cascade_scale,
        );

        if self.filter_across_cascades {
          // sample the next cascade and blend between the two results to
          // smooth the transition
          let blend_threshold = val(0.1);

          let distance = render_position
            .dot((camera_world_none_translation_mat.forward() * val(Vec3::splat(-1.))).normalize());
          let next_split = split_at_fn(split_distances, cascade_index);
          let prev_split = cascade_index.equals(val(0_u32)).select_branched(
            || val(0.),
            || split_at_fn(split_distances, cascade_index - val(1_u32)),
          );
          let fade_factor = (next_split - distance) / (next_split - prev_split);

          let should_fade = fade_factor
            .less_equal_than(blend_threshold)
            .and(cascade_index.less_than(val(CASCADE_SHADOW_SPLIT_COUNT as u32 - 1)));

          should_fade.select_branched(
            || {
              let next_index = cascade_index + val(1_u32);
              let next_cascade_info = shadow_info.map_info().index(next_index).load().expand();

              let next_shadow_position = compute_shadow_position(
                render_position,
                render_normal,
                shadow_world_position,
                camera_world_position,
                bias,
                next_cascade_info.map_info,
                next_cascade_info.shadow_center_without_translation_to_shadowmap_ndc,
                self.bias_behavior.use_n_dot_l_normal_offset,
                self.reversed_depth,
              );

              let next_occlusion = self.shadow_computer.compute_shadow(
                next_shadow_position,
                screen_position,
                next_cascade_info.map_info,
                next_cascade_info.cascade_scale,
              );

              let lerp_amt = fade_factor.smoothstep(val(0.), blend_threshold);
              lerp_amt.mix(next_occlusion, current_occlusion)
            },
            || current_occlusion,
          )
        } else {
          current_occlusion
        }
      },
      || val(1.0),
    )
  }
}

pub const DEBUG_SHADOW_UV: bool = false;
pub const DEBUG_CASCADE_INDEX: bool = false;

/// get the split distance at the given cascade index
#[shader_fn]
pub fn split_at(splits: Node<Vec4<f32>>, index: Node<u32>) -> Node<f32> {
  let result = zeroed_val::<f32>().make_local_var();
  switch_by(index)
    .case(0, || result.store(splits.x()))
    .case(1, || result.store(splits.y()))
    .case(2, || result.store(splits.z()))
    .case(3, || result.store(splits.w()))
    .end_with_default(|| result.store(splits.w()));
  result.load()
}

/// compute the current shading point in which sub frustum
#[shader_fn]
pub fn compute_cascade_index(
  render_position: Node<Vec3<f32>>,
  camera_world_none_translation_mat: Node<Mat4<f32>>,
  splits: Node<Vec4<f32>>,
) -> Node<u32> {
  let camera_forward_dir =
    (camera_world_none_translation_mat.forward() * val(Vec3::splat(-1.))).normalize();

  let diff = render_position;
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

  if DEBUG_CASCADE_INDEX {
    let offset = offset.load().into_f32() / val(4.0);
    DEFAULT_DISPLAY_DEBUG.with_borrow_mut(|v| {
      if let Some(v) = v {
        v.store(offset.splat());
      }
    })
  }

  offset.load()
}
