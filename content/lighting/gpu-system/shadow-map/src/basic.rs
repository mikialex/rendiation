use database::RawEntityHandle;
use fast_hash_collection::FastHashMap;
use rendiation_texture_packer::pack_2d_to_3d::RemappedGrowablePacker;

use crate::*;

pub struct BasicShadowMapInfoInput {
  pub light_world: Mat4<f64>,
  pub proj: Mat4<f32>,
  pub map_size: Size,
  pub bias: ShadowBias,
}

#[derive(Clone)]
pub struct BasicShadowMapGPU {
  pub shadow_map: ShadowAtlas,
  // scene entity -> per-scene uniform buffer
  pub uniforms: FastHashMap<RawEntityHandle, UniformArray<BasicShadowMapInfo, MAX_SHADOW_COUNT>>,
}

pub type LightArrayAllocateResult = FastHashMap<RawEntityHandle, FastHashMap<RawEntityHandle, u32>>;

/// shadow_info_access: light_id -> Option<BasicShadowMapInfo>, return None if no shadow
pub fn prepare_basic_shadow_map_uniform(
  atlas_config: &MultiLayerTexturePackerConfig,
  light_uniform_array_index_mapping: &LightArrayAllocateResult,
  shadow_info_access: &dyn Fn(RawEntityHandle) -> Option<BasicShadowMapInfoInput>,
  gpu_data: &mut Option<BasicShadowMapGPU>,
  gpu: &GPU,
) -> BasicShadowMapPreparer {
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
            shadow_info.proj * world_inv.remove_position().into_f32();

          BasicShadowMapInfo {
            enabled: Bool::from(true),
            map_info,
            bias: shadow_info.bias,
            shadow_world_position,
            shadow_center_without_translation_to_shadowmap_ndc,
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

  let (old_shadow_map, mut old_uniforms) = match gpu_data.as_ref() {
    Some(g) => (Some(g.shadow_map.clone()), g.uniforms.clone()),
    None => (None, FastHashMap::default()),
  };

  let required_size = packer.current_size();
  let shadow_map = match old_shadow_map {
    Some(existing) if existing.size() == required_size.into_gpu_size() => existing,
    _ => ShadowAtlas::new("basic-shadow-map-atlas", required_size.into_gpu_size(), gpu),
  };

  let uniforms: FastHashMap<RawEntityHandle, UniformArray<BasicShadowMapInfo, MAX_SHADOW_COUNT>> =
    new_shadow_info
      .iter()
      .map(|(scene_id, info)| {
        let uniform = if let Some(existing) = old_uniforms.remove(scene_id) {
          existing.write_at(&gpu.queue, info, 0);
          existing
        } else {
          create_uniform(info.clone(), &gpu.device, "basic-shadow-map-uniform")
        };
        (*scene_id, uniform)
      })
      .collect();

  *gpu_data = Some(BasicShadowMapGPU {
    shadow_map: shadow_map.clone(),
    uniforms: uniforms.clone(),
  });

  BasicShadowMapPreparer {
    gpu_data: BasicShadowMapGPU {
      shadow_map,
      uniforms,
    },
    source_world: source_world_map.into_boxed(),
    source_proj: source_proj_map.into_boxed(),
    packing: PackerView(Arc::new(packer)).into_boxed(),
  }
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

pub struct BasicShadowMapPreparer {
  pub gpu_data: BasicShadowMapGPU,
  source_world: BoxedDynQuery<RawEntityHandle, Mat4<f64>>,
  source_proj: BoxedDynQuery<RawEntityHandle, Mat4<f32>>,
  packing: BoxedDynQuery<RawEntityHandle, ShadowMapAddressInfo>,
}

impl BasicShadowMapPreparer {
  pub fn update_shadow_maps(
    self,
    frame_ctx: &mut FrameCtx,
    scene_content: &mut dyn FnMut(&mut FrameCtx, ShadowMapDrawRequest),
    reversed_depth: bool,
  ) -> BasicShadowMapGPU {
    clear_shadow_map(&self.gpu_data.shadow_map, frame_ctx, reversed_depth);

    // do shadowmap updates
    for (light_id, shadow_view) in self.packing.iter_key_value() {
      let shadow_camera_world = self.source_world.access(&light_id).unwrap();
      let shadow_camera_proj = self.source_proj.access(&light_id).unwrap();

      let write_view = self
        .gpu_data
        .shadow_map
        .get_layer_view(shadow_view.layer_index as u32)
        .clone();

      // todo, consider merge the pass within the same layer
      // custom dispatcher is not required because we only have depth output.
      let pass = pass("shadow-map").with_depth(
        &RenderTargetView::from_texture_view(write_view),
        load_and_store(),
        load_and_store(),
      );

      scene_content(
        frame_ctx,
        ShadowMapDrawRequest {
          shadow_camera_proj,
          shadow_camera_world,
          light_id,
          map_desc: ShadowPassDesc {
            desc: pass,
            address: shadow_view,
          },
        },
      );
    }

    self.gpu_data
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct BasicShadowMapInfo {
  pub enabled: Bool,
  pub shadow_center_without_translation_to_shadowmap_ndc: Mat4<f32>,
  pub shadow_world_position: HighPrecisionTranslationUniform,
  pub bias: ShadowBias,
  pub map_info: ShadowMapAddressInfo,
}

#[derive(Clone)]
pub struct BasicShadowMapComponent {
  pub shadow_map_atlas: GPU2DArrayDepthTextureView,
  pub info: UniformBufferDataView<Shader140Array<BasicShadowMapInfo, MAX_SHADOW_COUNT>>,
  pub reversed_depth: bool,
}

impl AbstractShaderBindingSource for BasicShadowMapComponent {
  type ShaderBindResult = BasicShadowMapInvocation;
  fn bind_shader(&self, cx: &mut ShaderBindGroupBuilder) -> BasicShadowMapInvocation {
    BasicShadowMapInvocation {
      shadow_map_atlas: cx.bind_by(&self.shadow_map_atlas),
      sampler: cx.bind_by(&ImmediateGPUCompareSamplerViewBind),
      info: cx.bind_by(&self.info),
    }
  }
}
impl AbstractBindingSource for BasicShadowMapComponent {
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    ctx.bind(&self.shadow_map_atlas);
    ctx.bind_immediate_sampler(&create_shadow_depth_sampler_desc(self.reversed_depth));
    ctx.bind(&self.info);
  }
}

#[derive(Clone)]
pub struct BasicShadowMapInvocation {
  shadow_map_atlas: BindingNode<ShaderDepthTexture2DArray>,
  sampler: BindingNode<ShaderCompareSampler>,
  info: ShaderReadonlyPtrOf<Shader140Array<BasicShadowMapInfo, MAX_SHADOW_COUNT>>,
}

impl BasicShadowMapInvocation {
  pub fn query_shadow_occlusion_by_idx(
    &self,
    render_position: Node<Vec3<f32>>,
    render_normal: Node<Vec3<f32>>,
    shadow_idx: Node<u32>,
    camera_world_position: Node<HighPrecisionTranslation>,
  ) -> Node<f32> {
    let enabled = self.info.index(shadow_idx).enabled().load();
    enabled.into_bool().select_branched(
      || {
        let shadow_info = self.info.index(shadow_idx).load().expand();
        let bias = shadow_info.bias.expand();

        // apply normal bias
        let render_position = render_position + bias.normal_bias * render_normal;

        let shadow_center_in_render_space = hpt_sub_hpt(
          hpt_uniform_to_hpt(shadow_info.shadow_world_position),
          camera_world_position,
        );

        let position_in_shadow_center_without_translation_space =
          render_position - shadow_center_in_render_space;

        let shadow_position = shadow_info.shadow_center_without_translation_to_shadowmap_ndc
          * (position_in_shadow_center_without_translation_space, val(1.)).into();

        let shadow_position = shadow_position.xyz() / shadow_position.w().splat();

        // convert to uv space and apply offset bias
        let shadow_position = shadow_position * val(Vec3::new(0.5, -0.5, 1.))
          + val(Vec3::new(0.5, 0.5, 0.))
          + (val(0.), val(0.), bias.bias).into();

        sample_shadow_pcf_x36_by_offset(
          self.shadow_map_atlas,
          shadow_position,
          self.sampler,
          shadow_info.map_info.expand(),
        )
      },
      || val(1.),
    )
  }
}

impl IntoShaderIterator for BasicShadowMapInvocation {
  type ShaderIter = BasicShadowMapInvocationIter;

  fn into_shader_iter(self) -> Self::ShaderIter {
    BasicShadowMapInvocationIter {
      iter: self.info.clone().into_shader_iter(),
      inner: self,
    }
  }
}

#[derive(Clone)]
pub struct BasicShadowMapInvocationIter {
  inner: BasicShadowMapInvocation,
  iter: ShaderStaticArrayReadonlyIter<
    Shader140Array<BasicShadowMapInfo, MAX_SHADOW_COUNT>,
    BasicShadowMapInfo,
  >,
}

impl ShaderIterator for BasicShadowMapInvocationIter {
  type Item = BasicShadowMapSingleInvocation;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let (valid, (index, _)) = self.iter.shader_next();

    let item = BasicShadowMapSingleInvocation {
      sys: self.inner.clone(),
      index,
    };

    (valid, item)
  }
}

#[derive(Clone)]
pub struct BasicShadowMapSingleInvocation {
  sys: BasicShadowMapInvocation,
  index: Node<u32>,
}

impl ShadowOcclusionQuery for BasicShadowMapSingleInvocation {
  fn query_shadow_occlusion(
    &self,
    render_position: Node<Vec3<f32>>,
    render_normal: Node<Vec3<f32>>,
    camera_world_position: Node<HighPrecisionTranslation>,
    _camera_world_none_translation_mat: Node<Mat4<f32>>,
  ) -> Node<f32> {
    self.sys.query_shadow_occlusion_by_idx(
      render_position,
      render_normal,
      self.index,
      camera_world_position,
    )
  }
}
