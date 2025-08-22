use std::sync::Arc;

use database::RawEntityHandle;
use fast_hash_collection::FastHashMap;
use parking_lot::RwLock;
use rendiation_texture_packer::pack_2d_to_3d::RemappedGrowablePacker;

use crate::*;

pub fn use_basic_shadow_map_uniform(
  cx: &mut QueryGPUHookCx,
  source_world: UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = Mat4<f64>>>,
  source_proj: UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = Mat4<f32>>>,
  size: UseResult<impl DataChanges<Key = RawEntityHandle, Value = Size> + 'static>,
  bias: UseResult<impl DataChanges<Key = u32, Value = ShadowBias> + 'static>,
  enabled: UseResult<impl DataChanges<Key = u32, Value = bool> + 'static>,
  atlas_config: MultiLayerTexturePackerConfig,
) -> Option<(BasicShadowMapPreparer, UniformArray<BasicShadowMapInfo, 8>)> {
  let (cx, uniform) = cx.use_uniform_array_buffers();

  let (source_world1, source_world2) = source_world.fork();
  let (source_world2, source_world3) = source_world2.fork();

  let (source_proj1, source_proj2) = source_proj.fork();

  let source_world_view = source_world1.use_retain_view_to_resolve_stage(cx);

  source_world2
    .into_delta_change()
    .use_assure_result(cx)
    .map_changes(|world_matrix| into_hpt(world_matrix.position()).into_uniform())
    .update_uniform_array(
      uniform,
      offset_of!(BasicShadowMapInfo, shadow_world_position),
      cx.gpu,
    );

  let source_proj_view = source_proj1.use_retain_view_to_resolve_stage(cx);

  source_world3
    .dual_query_zip(source_proj2)
    .dual_query_map(|(world_matrix, projection)| {
      let world_inv = world_matrix.inverse_or_identity();
      projection * world_inv.remove_position().into_f32()
    })
    .use_assure_result(cx)
    .into_delta_change()
    .update_uniform_array(
      uniform,
      offset_of!(
        BasicShadowMapInfo,
        shadow_center_to_shadowmap_ndc_without_translation
      ),
      cx.gpu,
    );

  enabled.map_changes(Bool::from).update_uniform_array(
    uniform,
    offset_of!(BasicShadowMapInfo, enabled),
    cx.gpu,
  );

  bias.update_uniform_array(uniform, offset_of!(BasicShadowMapInfo, bias), cx.gpu);

  // todo, spawn a task to pack
  let (cx, packer) =
    cx.use_plain_state(|| Arc::new(RwLock::new(RemappedGrowablePacker::new(atlas_config))));

  if let Some(size_changes) = size.if_ready() {
    let mut new_size = None;
    let mut buff_changes = FastHashMap::default();

    packer.write().process(
      size_changes.iter_removed(),
      size_changes.iter_update_or_insert(),
      |_new_size| {
        new_size = Some(_new_size);
      },
      |key, delta| {
        merge_change(&mut buff_changes, (key, delta));
      },
    );

    buff_changes
      .into_change()
      .collective_map(|v| v.map(convert_pack_result).unwrap_or_default()) // todo, handle allocation fail in shader access
      .update_uniform_array(uniform, offset_of!(BasicShadowMapInfo, map_info), cx.gpu);
  }

  let (cx, atlas) = cx.use_plain_state_default::<Option<GPU2DArrayDepthTextureView>>();

  cx.when_render(|| {
    let shadow_map_atlas = get_or_create_shadow_atlas(
      "basic-shadow-map-atlas",
      packer.read().current_size(),
      atlas,
      cx.gpu,
    );

    let system = BasicShadowMapPreparer {
      shadow_map_atlas,
      source_world: source_world_view.expect_resolve_stage().into_boxed(),
      source_proj: source_proj_view.expect_resolve_stage().into_boxed(),
      packing: PackerView(packer.clone().make_read_holder()).into_boxed(),
    };

    (system, uniform.clone())
  })
}

#[derive(Clone)]
struct PackerView(LockReadGuardHolder<RemappedGrowablePacker<RawEntityHandle>>);

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
}

pub struct BasicShadowMapPreparer {
  shadow_map_atlas: GPU2DArrayDepthTextureView,
  source_world: BoxedDynQuery<RawEntityHandle, Mat4<f64>>,
  source_proj: BoxedDynQuery<RawEntityHandle, Mat4<f32>>,
  packing: BoxedDynQuery<RawEntityHandle, ShadowMapAddressInfo>,
}

impl BasicShadowMapPreparer {
  pub fn update_shadow_maps(
    self,
    frame_ctx: &mut FrameCtx,
    // proj, world
    scene_content: &impl Fn(Mat4<f32>, Mat4<f64>, &mut FrameCtx, ShadowPassDesc),
    reversed_depth: bool,
  ) -> GPU2DArrayDepthTextureView {
    clear_shadow_map(&self.shadow_map_atlas, frame_ctx, reversed_depth);

    // do shadowmap updates
    for (idx, shadow_view) in self.packing.iter_key_value() {
      let world = self.source_world.access(&idx).unwrap();
      let proj = self.source_proj.access(&idx).unwrap();

      let write_view = self
        .shadow_map_atlas
        .resource
        .create_view(TextureViewDescriptor {
          label: Some("shadowmap-write-view"),
          dimension: Some(TextureViewDimension::D2),
          base_array_layer: shadow_view.layer_index as u32,
          array_layer_count: Some(1),
          ..Default::default()
        });

      // todo, consider merge the pass within the same layer
      // custom dispatcher is not required because we only have depth output.
      let pass =
        pass("shadow-map").with_depth(&RenderTargetView::Texture(write_view), load_and_store());

      scene_content(
        proj,
        world,
        frame_ctx,
        ShadowPassDesc {
          desc: pass,
          address: shadow_view,
        },
      );
    }

    self.shadow_map_atlas
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct BasicShadowMapInfo {
  pub enabled: Bool,
  pub shadow_center_to_shadowmap_ndc_without_translation: Mat4<f32>,
  pub shadow_world_position: HighPrecisionTranslationUniform,
  pub bias: ShadowBias,
  pub map_info: ShadowMapAddressInfo,
}

#[derive(Clone)]
pub struct BasicShadowMapComponent {
  pub shadow_map_atlas: GPU2DArrayDepthTextureView,
  pub info: UniformBufferDataView<Shader140Array<BasicShadowMapInfo, 8>>,
  pub reversed_depth: bool,
}

impl AbstractBindingSource for BasicShadowMapComponent {
  type ShaderBindResult = BasicShadowMapInvocation;
  fn bind_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.shadow_map_atlas);
    ctx.bind_immediate_sampler(&create_shadow_depth_sampler_desc(self.reversed_depth));
    ctx.binding.bind(&self.info);
  }

  fn bind_shader(&self, cx: &mut ShaderBindGroupBuilder) -> BasicShadowMapInvocation {
    BasicShadowMapInvocation {
      shadow_map_atlas: cx.bind_by(&self.shadow_map_atlas),
      sampler: cx.bind_by(&ImmediateGPUCompareSamplerViewBind),
      info: cx.bind_by(&self.info),
    }
  }
}

#[derive(Clone)]
pub struct BasicShadowMapInvocation {
  shadow_map_atlas: BindingNode<ShaderDepthTexture2DArray>,
  sampler: BindingNode<ShaderCompareSampler>,
  info: ShaderReadonlyPtrOf<Shader140Array<BasicShadowMapInfo, 8>>,
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

        let position_in_shadow_center_space_without_translation =
          render_position - shadow_center_in_render_space;

        let shadow_position = shadow_info.shadow_center_to_shadowmap_ndc_without_translation
          * (position_in_shadow_center_space_without_translation, val(1.)).into();

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
  iter: ShaderStaticArrayReadonlyIter<Shader140Array<BasicShadowMapInfo, 8>, BasicShadowMapInfo>,
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
