use database::RawEntityHandle;
use fast_hash_collection::FastHashMap;
use rendiation_texture_packer::pack_2d_to_3d::RemappedGrowablePacker;

use crate::*;

pub struct BasicShadowMapInfoInput {
  pub light_world: Mat4<f64>,
  pub proj: ShadowCameraProjectionMatrixes,
  pub map_size: Size,
  pub bias: ShadowBias,
}

#[derive(Clone, Default)]
pub struct BasicShadowMapInfoGPU {
  // scene entity -> per-scene uniform buffer
  pub uniforms: FastHashMap<RawEntityHandle, UniformArray<BasicShadowMapInfo, MAX_SHADOW_COUNT>>,
}

pub type LightArrayAllocateResult = FastHashMap<RawEntityHandle, FastHashMap<RawEntityHandle, u32>>;

/// shadow_info_access: light_id -> Option<BasicShadowMapInfo>, return None if no shadow
///
/// return (preparer, shadow_map_atlas_size_require)
pub fn prepare_basic_shadow_map_uniform(
  atlas_config: &MultiLayerTexturePackerConfig,
  light_uniform_array_index_mapping: &LightArrayAllocateResult,
  shadow_info_access: &dyn Fn(RawEntityHandle) -> Option<BasicShadowMapInfoInput>,
  gpu_data: &mut Option<BasicShadowMapInfoGPU>,
  gpu: &GPU,
) -> (BasicShadowMapPreparer, SizeWithDepth) {
  let mut packer = RemappedGrowablePacker::<RawEntityHandle>::new(*atlas_config);
  let mut source_world_map = FastHashMap::default();
  let mut source_proj_map = FastHashMap::default();

  let new_shadow_info: FastHashMap<
    RawEntityHandle,
    Shader140Array<BasicShadowMapInfo, MAX_SHADOW_COUNT>,
  > = light_uniform_array_index_mapping
    .iter()
    .map(|(scene_id, light_id_mapping)| {
      let mut shadow_info_array = Shader140Array::<BasicShadowMapInfo, MAX_SHADOW_COUNT>::default();

      // packer maybe resize, so we have to batch process first
      let sizes = light_id_mapping
        .iter()
        .filter_map(|(light_id, _)| shadow_info_access(*light_id).map(|v| (*light_id, v.map_size)));
      packer.process([].into_iter(), sizes, |_| {}, |_, _| {});

      //
      for (light_id, uniform_array_index) in light_id_mapping.iter() {
        let shadow_uniform = if let Some(shadow_info) = shadow_info_access(*light_id) {
          // todo, handle allocation fail(warning and handle shader access)
          let map_info = packer
            .access(light_id)
            .unwrap()
            .map(convert_pack_result)
            .unwrap_or(Default::default());

          source_world_map.insert(*light_id, shadow_info.light_world);
          source_proj_map.insert(*light_id, shadow_info.proj);

          let world_mat = shadow_info.light_world;
          let shadow_world_position = into_hpt(world_mat.position()).into_uniform();

          let world_inv = world_mat.inverse_or_identity();
          let shadow_center_without_translation_to_shadowmap_ndc =
            shadow_info.proj.render_matrix * world_inv.remove_position().into_f32();

          BasicShadowMapInfo {
            enabled: Bool::from(true),
            map_info,
            bias: shadow_info.bias,
            shadow_world_position,
            shadow_center_without_translation_to_shadowmap_ndc,
            shadow_proj_linear_depth_recover_helper:
              extract_shadow_proj_linear_depth_recover_helper(shadow_info.proj.opengl_ndc_matrix),
            ..Default::default()
          }
        } else {
          BasicShadowMapInfo {
            enabled: Bool::from(false),
            ..Default::default()
          }
        };
        shadow_info_array.set(*uniform_array_index as usize, shadow_uniform);
      }
      (*scene_id, shadow_info_array)
    })
    .collect();

  let uniforms = gpu_data.get_or_insert_default();

  let uniforms: FastHashMap<RawEntityHandle, UniformArray<BasicShadowMapInfo, MAX_SHADOW_COUNT>> =
    new_shadow_info
      .iter()
      .map(|(scene_id, info)| {
        let uniform = if let Some(existing) = uniforms.uniforms.remove(scene_id) {
          existing.write_at(&gpu.queue, info, 0);
          existing
        } else {
          create_uniform(info.clone(), &gpu.device, "basic-shadow-map-uniform")
        };
        (*scene_id, uniform)
      })
      .collect();

  *gpu_data = Some(BasicShadowMapInfoGPU {
    uniforms: uniforms.clone(),
  });

  let required_size = packer.current_size();

  (
    BasicShadowMapPreparer {
      gpu_data: BasicShadowMapInfoGPU { uniforms },
      source_world: source_world_map.into_boxed(),
      source_proj: source_proj_map.into_boxed(),
      packing: PackerView(Arc::new(packer)).into_boxed(),
    },
    required_size,
  )
}

#[derive(Clone)]
struct PackerView(Arc<RemappedGrowablePacker<RawEntityHandle>>);

impl Query for PackerView {
  type Key = RawEntityHandle;
  type Value = ShadowMapAddressInfo;

  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self
      .0
      .iter_key_value()
      .filter_map(|(k, v)| (k, convert_pack_result(v?)).into())
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    self.0.access(key)?.map(convert_pack_result)
  }

  fn has_item_hint(&self) -> bool {
    !self.0.is_empty()
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ShadowCameraProjectionMatrixes {
  /// ndc is webgpu and considered reverse depth config
  pub render_matrix: Mat4<f32>,
  pub opengl_ndc_matrix: Mat4<f32>,
}

/// extract the helper for recovering the linear depth from the render space
/// ndc depth, the proj must be the opengl ndc projection of the shadow camera,
/// the w row values are 0/1 for the orthographic projection and -1/0 for the
/// perspective projection, see recover_linear_depth
pub fn extract_shadow_proj_linear_depth_recover_helper(
  proj: Mat4<f32>,
) -> ProjLinearDepthRecoverHelper {
  let (near, far) = proj.get_near_far_assume_is_common_projection();
  ProjLinearDepthRecoverHelper {
    near,
    far,
    w_row: Vec2::new(proj.c4, proj.d4),
    ..Default::default()
  }
}

pub struct BasicShadowMapPreparer {
  pub gpu_data: BasicShadowMapInfoGPU,
  source_world: BoxedDynQuery<RawEntityHandle, Mat4<f64>>,
  source_proj: BoxedDynQuery<RawEntityHandle, ShadowCameraProjectionMatrixes>,
  packing: BoxedDynQuery<RawEntityHandle, ShadowMapAddressInfo>,
}

impl BasicShadowMapPreparer {
  pub fn update_shadow_maps(
    self,
    frame_ctx: &mut FrameCtx,
    shadow_map: &mut dyn AbstractShadowMapGPUData,
    scene_content: &mut dyn FnMut(&mut FrameCtx, ShadowMapDrawRequest),
  ) -> BasicShadowMapInfoGPU {
    shadow_map.clear_shadow_map(frame_ctx);

    // do shadowmap updates
    for (light_id, address) in self.packing.iter_key_value() {
      let shadow_camera_world = self.source_world.access(&light_id).unwrap();
      let shadow_camera_proj = self.source_proj.access(&light_id).unwrap();

      let request = ShadowMapUpdateRequest {
        shadow_camera_proj,
        shadow_camera_world,
        light_id,
        address,
      };

      // todo, consider merge the pass within the same layer
      shadow_map.update_shadow_map(frame_ctx, request, scene_content);
    }

    self.gpu_data
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct BasicShadowMapInfo {
  pub enabled: Bool,
  pub shadow_proj_linear_depth_recover_helper: ProjLinearDepthRecoverHelper,
  pub shadow_center_without_translation_to_shadowmap_ndc: Mat4<f32>,
  pub shadow_world_position: HighPrecisionTranslationUniform,
  pub bias: ShadowBias,
  pub map_info: ShadowMapAddressInfo,
}

#[derive(Clone)]
pub struct BasicShadowMapComponent {
  pub info: UniformBufferDataView<Shader140Array<BasicShadowMapInfo, MAX_SHADOW_COUNT>>,
  pub bias_behavior: ShadowBiasBehaviorConfig,
  pub reversed_depth: bool,
  pub shadow_computer: Arc<dyn AbstractShadowComputer>,
}

impl ShaderHashProvider for BasicShadowMapComponent {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(&self.reversed_depth);
    hasher.hash(&self.bias_behavior);
    self.shadow_computer.hash_pipeline(hasher);
  }
}

impl AbstractShaderBindingSource for BasicShadowMapComponent {
  type ShaderBindResult = BasicShadowMapInvocation;
  fn bind_shader(&self, cx: &mut ShaderBindGroupBuilder) -> BasicShadowMapInvocation {
    BasicShadowMapInvocation {
      shadow_computer: self.shadow_computer.bind_shader(cx),
      info: cx.bind_by(&self.info),
      bias_behavior: self.bias_behavior,
      reversed_depth: self.reversed_depth,
    }
  }
}

impl AbstractBindingSource for BasicShadowMapComponent {
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    self.shadow_computer.bind_pass(ctx);
    ctx.bind(&self.info);
  }
}

pub struct BasicShadowMapInvocation {
  shadow_computer: Box<dyn AbstractShadowComputerInvocation>,
  info: ShaderReadonlyPtrOf<Shader140Array<BasicShadowMapInfo, MAX_SHADOW_COUNT>>,
  bias_behavior: ShadowBiasBehaviorConfig,
  reversed_depth: bool,
}

impl BasicShadowMapInvocation {
  pub fn query_shadow_occlusion_by_idx(
    &self,
    render_position: Node<Vec3<f32>>,
    render_normal: Node<Vec3<f32>>,
    shadow_idx: Node<u32>,
    screen_position: Node<Vec2<f32>>,
    camera_world_position: Node<HighPrecisionTranslation>,
  ) -> Node<f32> {
    let enabled = self.info.index(shadow_idx).enabled().load();
    enabled.into_bool().select_branched(
      || {
        let shadow_info = self.info.index(shadow_idx);
        let map_info = shadow_info.map_info().load();

        let shadow_position = compute_shadow_position(
          render_position,
          render_normal,
          shadow_info.shadow_world_position().load(),
          camera_world_position,
          shadow_info.bias().load(),
          map_info,
          shadow_info
            .shadow_center_without_translation_to_shadowmap_ndc()
            .load(),
          self.bias_behavior.use_n_dot_l_normal_offset,
          self.reversed_depth,
        );

        self.shadow_computer.compute_shadow(
          shadow_position,
          screen_position,
          map_info,
          val(1.),
          shadow_info.shadow_proj_linear_depth_recover_helper(),
        )
      },
      || val(1.),
    )
  }
}

pub fn compute_shadow_position(
  render_position: Node<Vec3<f32>>,
  render_normal: Node<Vec3<f32>>,
  shadow_world_position: Node<HighPrecisionTranslationUniform>,
  camera_world_position: Node<HighPrecisionTranslation>,
  bias: Node<ShadowBias>,
  map_info: Node<ShadowMapAddressInfo>,
  shadow_center_without_translation_to_shadowmap_ndc: Node<Mat4<f32>>,
  use_n_dot_l_normal_offset: bool,
  reversed_depth: bool,
) -> Node<Vec3<f32>> {
  let bias = bias.expand();

  let shadow_center_in_render_space = hpt_sub_hpt(
    hpt_uniform_to_hpt(shadow_world_position),
    camera_world_position,
  );

  let position_in_shadow_center_without_translation_space =
    render_position - shadow_center_in_render_space;

  // the normal bias is in texel units, so that it scales with the shadow
  // map resolution, note for perspective projections this is the texel
  // size at the near plane
  let texel_world_size = shadow_texel_world_size_fn(
    shadow_center_without_translation_to_shadowmap_ndc,
    position_in_shadow_center_without_translation_space,
    map_info,
  );

  // apply normal bias, optionally scaled by (1 - nDotL) to be smaller on the lit side
  let normal_offset = compute_normal_offset(
    position_in_shadow_center_without_translation_space,
    render_normal,
    texel_world_size,
    bias.normal_bias,
    use_n_dot_l_normal_offset,
  );

  let shadow_position = shadow_center_without_translation_to_shadowmap_ndc
    * (
      position_in_shadow_center_without_translation_space + normal_offset,
      val(1.),
    )
      .into();

  let shadow_position = shadow_position.xyz() / shadow_position.w().splat();

  // convert to uv space
  let shadow_position =
    shadow_position * val(Vec3::new(0.5, -0.5, 1.)) + val(Vec3::new(0.5, 0.5, 0.));

  apply_direct_depth_bias(reversed_depth, bias.bias, shadow_position)
}
