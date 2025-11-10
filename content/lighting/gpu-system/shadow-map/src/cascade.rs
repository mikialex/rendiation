use rendiation_texture_packer::{
  pack_2d_to_2d::pack_impl::etagere_wrap::EtagerePacker, pack_2d_to_3d::MultiLayerTexturePackerRaw,
  TexturePacker, TexturePackerInit,
};

use crate::*;

const CASCADE_SHADOW_SPLIT_COUNT: usize = 4;

/// As the cascade shadow map is highly dynamic(the change related to camera) and
/// the shadow count should be small, the implementation is none-incremental
pub struct CascadeShadowMapSystemInputs {
  /// alloc_id => shadow map world
  pub source_world: BoxedDynQuery<u32, Mat4<f64>>,
  /// alloc_id => shadow map proj
  pub source_proj: BoxedDynQuery<u32, Mat4<f32>>,
  /// alloc_id => shadow map resolution
  pub size: BoxedDynQuery<u32, Size>,
  /// alloc_id => shadow map bias
  pub bias: BoxedDynQuery<u32, ShadowBias>,
  /// alloc_id => enabled
  pub enabled: BoxedDynQuery<u32, bool>,
}

type CascadeShadowPackerImpl = MultiLayerTexturePackerRaw<EtagerePacker>;

pub fn generate_cascade_shadow_info(
  inputs: &CascadeShadowMapSystemInputs,
  packer_size: SizeWithDepth,
  view_camera_proj: Mat4<f32>,
  view_camera_world: Mat4<f64>,
  ndc: &dyn NDCSpaceMapper<f32>,
) -> CascadeShadowPreparer {
  let mut packer = CascadeShadowPackerImpl::init_by_config(packer_size);

  let mut gpu_buffer = Vec::new();
  let mut proj_info = Vec::new();

  for (k, enabled) in inputs.enabled.iter_key_value() {
    let gpu_buffer_idx = k as usize;
    gpu_buffer.resize(gpu_buffer.len().max(gpu_buffer_idx + 1), Default::default());
    proj_info.resize(proj_info.len().max(gpu_buffer_idx + 1), Default::default());

    if !enabled {
      continue;
    }

    let world = inputs.source_world.access(&k).unwrap();
    let mut sub_proj_info = [Mat4::default(); CASCADE_SHADOW_SPLIT_COUNT];

    let world_to_light =
      inputs.source_proj.access(&k).unwrap().into_f64() * world.inverse_or_identity();

    let cascades =
      compute_directional_light_cascade_info(view_camera_world, view_camera_proj, world_to_light);

    let light_world = inputs.source_world.access(&k).unwrap();
    let light_world_inv = light_world.inverse_or_identity();

    let mut cascade_info = Vec::<SingleShadowMapInfo>::with_capacity(CASCADE_SHADOW_SPLIT_COUNT);
    let size = inputs.size.access(&k).unwrap();
    let mut splits = [0.; CASCADE_SHADOW_SPLIT_COUNT];

    for (idx, (sub_proj, split)) in cascades.iter().enumerate() {
      if let Ok(pack) = packer.pack(size) {
        let proj = sub_proj.compute_projection_mat(ndc);
        let shadow_center_to_shadowmap_ndc_without_translation =
          proj * light_world_inv.remove_position().into_f32();

        cascade_info.push(SingleShadowMapInfo {
          map_info: convert_pack_result(pack),
          shadow_center_to_shadowmap_ndc_without_translation,
          ..Default::default()
        });
        splits[idx] = *split;
        sub_proj_info[idx] = proj;
      } else {
        continue;
      }
    }

    let shadow_world_position = into_hpt(light_world.position()).into_uniform();

    gpu_buffer[gpu_buffer_idx] = CascadeShadowMapInfo {
      bias: inputs.bias.access(&k).unwrap(),
      shadow_world_position,
      map_info: Shader140Array::from_slice_clamp_or_default(&cascade_info),
      splits: splits.into(),
      enabled: true.into(),
      ..Default::default()
    };

    proj_info[gpu_buffer_idx] = (world, sub_proj_info);
  }

  CascadeShadowPreparer {
    uniforms: gpu_buffer,
    map_size: packer_size,
    proj_info,
  }
}

pub struct CascadeShadowPreparer {
  uniforms: Vec<CascadeShadowMapInfo>,
  proj_info: Vec<(Mat4<f64>, [Mat4<f32>; 4])>,
  map_size: SizeWithDepth,
}

#[derive(Default)]
pub struct CascadeShadowGPUCache {
  texture: Option<GPU2DArrayDepthTextureView>,
  uniforms: Option<UniformBufferDataView<Shader140Array<CascadeShadowMapInfo, 8>>>,
}

impl CascadeShadowPreparer {
  #[must_use]
  pub fn update_shadow_maps(
    self,
    resource_cache: &mut CascadeShadowGPUCache,
    frame_ctx: &mut FrameCtx,
    // proj, world
    scene_content: &impl Fn(Mat4<f32>, Mat4<f64>, &mut FrameCtx, ShadowPassDesc),
    reversed_depth: bool,
  ) -> CascadeShadowMapComponent {
    let shadow_map_atlas = &mut resource_cache.texture;
    let uniforms = &mut resource_cache.uniforms;

    let shadow_map_atlas = get_or_create_shadow_atlas(
      "cascade-shadow-map-atlas",
      self.map_size,
      shadow_map_atlas,
      frame_ctx.gpu,
    );
    clear_shadow_map(&shadow_map_atlas, frame_ctx, reversed_depth);

    // do shadowmap updates
    for (index, cascade) in self.uniforms.iter().enumerate() {
      if cascade.enabled == Bool::from(false) {
        continue;
      }
      let proj_info = self.proj_info[index];
      let world = proj_info.0;

      for (slice_index, shadow_view) in cascade.map_info.iter().enumerate() {
        let shadow_view = shadow_view.map_info;

        let write_view = shadow_map_atlas
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
        let pass = pass("cascade-shadow-map")
          .with_depth(&RenderTargetView::Texture(write_view), load_and_store());

        let proj = proj_info.1[slice_index];

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
    }

    let info = uniforms.get_or_insert_with(|| {
      let uniforms = Shader140Array::from_slice_clamp_or_default(&self.uniforms);
      create_uniform(uniforms, &frame_ctx.gpu.device)
    });

    CascadeShadowMapComponent {
      shadow_map_atlas,
      info: info.clone(),
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
  pub shadow_center_to_shadowmap_ndc_without_translation: Mat4<f32>,
  pub split_distance: f32,
}

/// return per sub frustum light shadow camera projection mat and split distance
pub fn compute_directional_light_cascade_info(
  camera_world: Mat4<f64>,
  camera_projection: Mat4<f32>,
  world_to_light: Mat4<f64>,
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
  camera_world: Mat4<f64>,
  camera_projection: Mat4<f32>,
  world_to_light: Mat4<f64>,
) -> [(Vec3<f32>, Vec3<f32>, f32); CASCADE_SHADOW_SPLIT_COUNT] {
  let (near, far) = camera_projection.get_near_far_assume_is_common_projection();

  let world_to_clip = camera_projection.into_f64() * camera_world;
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
      let corner_position = corner_pair.0.lerp(corner_pair.1, ratio.into());

      let corner_position_in_light = world_to_light * corner_position;

      min = min.min(corner_position_in_light.into_f32());
      max = max.max(corner_position_in_light.into_f32());
    }
    idx += 1;
    (min, max, split_distance)
  })
}

#[derive(Clone)]
pub struct CascadeShadowMapComponent {
  pub shadow_map_atlas: GPU2DArrayDepthTextureView,
  pub info: UniformBufferDataView<Shader140Array<CascadeShadowMapInfo, 8>>,
  pub reversed_depth: bool,
}

impl ShaderHashProvider for CascadeShadowMapComponent {
  shader_hash_type_id! {}
}

impl AbstractShaderBindingSource for CascadeShadowMapComponent {
  type ShaderBindResult = CascadeShadowMapInvocation;
  fn bind_shader(&self, cx: &mut ShaderBindGroupBuilder) -> CascadeShadowMapInvocation {
    CascadeShadowMapInvocation {
      shadow_map_atlas: cx.bind_by(&self.shadow_map_atlas),
      sampler: cx.bind_by(&ImmediateGPUCompareSamplerViewBind),
      info: cx.bind_by(&self.info),
    }
  }
}
impl AbstractBindingSource for CascadeShadowMapComponent {
  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    ctx.bind(&self.shadow_map_atlas);
    ctx.bind_immediate_sampler(&create_shadow_depth_sampler_desc(self.reversed_depth));
    ctx.bind(&self.info);
  }
}

impl RandomAccessShadowProvider for CascadeShadowMapComponent {
  fn bind_shader(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn RandomAccessShadowProviderInvocation> {
    Box::new(AbstractShaderBindingSource::bind_shader(self, cx))
  }

  fn bind_pass(&self, cx: &mut BindingBuilder) {
    AbstractBindingSource::bind_pass(self, cx)
  }
}

#[derive(Clone)]
pub struct CascadeShadowMapInvocation {
  shadow_map_atlas: BindingNode<ShaderDepthTexture2DArray>,
  sampler: BindingNode<ShaderCompareSampler>,
  info: ShaderReadonlyPtrOf<Shader140Array<CascadeShadowMapInfo, 8>>,
}

impl RandomAccessShadowProviderInvocation for CascadeShadowMapInvocation {
  fn get_shadow_by_light_id(&self, light_id: Node<u32>) -> Box<dyn ShadowOcclusionQuery> {
    Box::new(CascadeShadowMapSingleInvocation {
      sys: self.clone(),
      index: light_id,
    })
  }
}

#[derive(Clone)]
pub struct CascadeShadowMapSingleInvocation {
  sys: CascadeShadowMapInvocation,
  index: Node<u32>,
}

impl ShadowOcclusionQuery for CascadeShadowMapSingleInvocation {
  fn query_shadow_occlusion(
    &self,
    render_position: Node<Vec3<f32>>,
    render_normal: Node<Vec3<f32>>,
    camera_world_position: Node<HighPrecisionTranslation>,
    camera_world_none_translation_mat: Node<Mat4<f32>>,
  ) -> Node<f32> {
    self.sys.query_shadow_occlusion_by_idx(
      render_position,
      render_normal,
      self.index,
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
    camera_world_position: Node<HighPrecisionTranslation>,
    camera_world_none_translation_mat: Node<Mat4<f32>>,
  ) -> Node<f32> {
    let enabled = self.info.index(shadow_idx).enabled().load();
    enabled.into_bool().select_branched(
      || {
        let shadow_info = self.info.index(shadow_idx);

        let bias = shadow_info.bias().load().expand();

        // apply normal bias
        let render_position = render_position + bias.normal_bias * render_normal;

        let shadow_center_in_render_space = hpt_sub_hpt(
          hpt_uniform_to_hpt(shadow_info.shadow_world_position().load()),
          camera_world_position,
        );

        let position_in_shadow_center_space_without_translation =
          render_position - shadow_center_in_render_space;

        let cascade_index = compute_cascade_index(
          render_position,
          camera_world_none_translation_mat,
          shadow_info.splits().load(),
        );

        let cascade_info = shadow_info.map_info().index(cascade_index).load().expand();

        let shadow_position = cascade_info.shadow_center_to_shadowmap_ndc_without_translation
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
          cascade_info.map_info.expand(),
        )
      },
      || val(1.0),
    )
  }
}

/// compute the current shading point in which sub frustum
#[shader_fn]
pub fn compute_cascade_index(
  render_position: Node<Vec3<f32>>,
  camera_world_none_translation_mat: Node<Mat4<f32>>,
  splits: Node<Vec4<f32>>,
) -> Node<u32> {
  let camera_forward_dir = camera_world_none_translation_mat.forward().normalize();

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

  offset.load()
}
