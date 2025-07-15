use std::mem::offset_of;
use std::task::{Context, Poll};

use reactive::*;
use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_texture_core::*;
use rendiation_texture_packer::pack_2d_to_3d::reactive_pack_2d_to_3d;
pub use rendiation_texture_packer::pack_2d_to_3d::{
  MultiLayerTexturePackerConfig, PackResult2dWithDepth,
};
use rendiation_webgpu::*;
use rendiation_webgpu_reactive_utils::*;

mod cascade;
pub use cascade::*;

mod map_utils;
use map_utils::*;

pub struct ShadowMapSystemInputs {
  /// alloc_id => shadow map world
  pub source_world: BoxedDynReactiveQuery<u32, Mat4<f64>>,
  /// alloc_id => shadow map proj
  pub source_proj: BoxedDynReactiveQuery<u32, Mat4<f32>>,
  /// alloc_id => shadow map resolution
  pub size: BoxedDynReactiveQuery<u32, Size>,
  /// alloc_id => shadow map bias
  pub bias: BoxedDynReactiveQuery<u32, ShadowBias>,
  /// alloc_id => enabled
  pub enabled: BoxedDynReactiveQuery<u32, bool>,
}

pub fn basic_shadow_map_uniform(
  inputs: ShadowMapSystemInputs,
  config: MultiLayerTexturePackerConfig,
  gpu_ctx: &GPU,
) -> (
  BasicShadowMapSystem,
  UniformArrayUpdateContainer<BasicShadowMapInfo, 8>,
) {
  let source_world = inputs.source_world.into_forker();

  let source_proj = inputs.source_proj.into_forker();

  let shadow_mat = source_world
    .clone()
    .collective_zip(source_proj.clone())
    .collective_map(|(world_matrix, projection)| {
      let world_inv = world_matrix.inverse_or_identity();
      projection * world_inv.remove_position().into_f32()
    })
    .into_boxed();

  let position = source_world
    .clone()
    .collective_map(|world_matrix| into_hpt(world_matrix.position()).into_uniform());

  let (sys, address) = BasicShadowMapSystem::new(
    config,
    source_world.into_boxed(),
    source_proj.into_boxed(),
    inputs.size,
  );

  let enabled = inputs
    .enabled
    .collective_map(|v| if v { 1 } else { 0 })
    .into_query_update_uniform_array(offset_of!(BasicShadowMapInfo, enabled), gpu_ctx);

  let map_info =
    address.into_query_update_uniform_array(offset_of!(BasicShadowMapInfo, map_info), gpu_ctx);

  let bias = inputs
    .bias
    .into_query_update_uniform_array(offset_of!(BasicShadowMapInfo, bias), gpu_ctx);

  let shadow_mat = shadow_mat.into_query_update_uniform_array(
    offset_of!(
      BasicShadowMapInfo,
      shadow_center_to_shadowmap_ndc_without_translation
    ),
    gpu_ctx,
  );

  let position = position.into_query_update_uniform_array(
    offset_of!(BasicShadowMapInfo, shadow_world_position),
    gpu_ctx,
  );

  let uniforms = UniformBufferDataView::create_default(&gpu_ctx.device);
  let uniforms = UniformArrayUpdateContainer::<BasicShadowMapInfo, 8>::new(uniforms)
    .with_source(enabled)
    .with_source(map_info)
    .with_source(shadow_mat)
    .with_source(position)
    .with_source(bias);

  (sys, uniforms)
}

pub struct BasicShadowMapSystem {
  shadow_map_atlas: Option<GPU2DArrayDepthTextureView>,
  packing: BoxedDynReactiveQuery<u32, ShadowMapAddressInfo>,
  atlas_resize: Box<dyn Stream<Item = SizeWithDepth> + Unpin>,
  current_size: Option<SizeWithDepth>,
  source_world: BoxedDynReactiveQuery<u32, Mat4<f64>>,
  source_proj: BoxedDynReactiveQuery<u32, Mat4<f32>>,
}

impl BasicShadowMapSystem {
  pub fn new(
    config: MultiLayerTexturePackerConfig,
    source_world: BoxedDynReactiveQuery<u32, Mat4<f64>>,
    source_proj: BoxedDynReactiveQuery<u32, Mat4<f32>>,
    size: BoxedDynReactiveQuery<u32, Size>,
  ) -> (Self, BoxedDynReactiveQuery<u32, ShadowMapAddressInfo>) {
    let (packing, atlas_resize) = reactive_pack_2d_to_3d(config, size);
    let packing = packing.collective_map(convert_pack_result).into_forker();

    let sys = Self {
      shadow_map_atlas: None,
      current_size: None,
      packing: packing.clone().into_boxed(),
      atlas_resize: Box::new(atlas_resize),
      source_world,
      source_proj,
    };
    (sys, packing.into_boxed())
  }

  #[must_use]
  pub fn update_shadow_maps(
    &mut self,
    cx: &mut Context,
    frame_ctx: &mut FrameCtx,
    // proj, world
    scene_content: &impl Fn(Mat4<f32>, Mat4<f64>, &mut FrameCtx, ShadowPassDesc),
    reversed_depth: bool,
  ) -> GPU2DArrayDepthTextureView {
    let (_, current_layouts) = self.packing.describe(cx).resolve_kept(); // incremental detail is useless here
    while let Poll::Ready(Some(new_size)) = self.atlas_resize.poll_next_unpin(cx) {
      // if we do shadow cache, we should also do content copy
      self.current_size = Some(new_size);
    }

    let shadow_map_atlas = get_or_create_map_with_init_clear(
      "basic-shadow-map-atlas",
      self.current_size.unwrap(),
      &mut self.shadow_map_atlas,
      frame_ctx,
      reversed_depth,
    );

    let (_, world) = self.source_world.describe(cx).resolve_kept();
    let (_, proj) = self.source_proj.describe(cx).resolve_kept();

    // do shadowmap updates
    for (idx, shadow_view) in current_layouts.iter_key_value() {
      let world = world.access(&idx).unwrap();
      let proj = proj.access(&idx).unwrap();

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

    shadow_map_atlas.clone()
  }
}

pub struct ShadowPassDesc {
  desc: RenderPassDescription,
  address: ShadowMapAddressInfo,
}

impl ShadowPassDesc {
  #[must_use]
  pub fn render_ctx(self, ctx: &mut FrameCtx) -> ActiveRenderPass {
    let mut pass = self.desc.render_ctx(ctx);

    let raw_pass = &mut pass.pass.ctx.pass;
    let x = self.address.offset.x;
    let y = self.address.offset.y;
    let w = self.address.size.x;
    let h = self.address.size.y;
    raw_pass.set_viewport(x, y, w, h, 0., 1.);

    pass
  }
}

pub fn convert_pack_result(r: PackResult2dWithDepth) -> ShadowMapAddressInfo {
  ShadowMapAddressInfo {
    layer_index: r.depth as i32,
    size: Vec2::new(
      usize::from(r.result.range.size.width) as f32,
      usize::from(r.result.range.size.height) as f32,
    ),
    offset: Vec2::new(
      r.result.range.origin.x as f32,
      r.result.range.origin.y as f32,
    ),
    ..Default::default()
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug)]
pub struct BasicShadowMapInfo {
  pub enabled: u32,
  pub shadow_center_to_shadowmap_ndc_without_translation: Mat4<f32>,
  pub shadow_world_position: HighPrecisionTranslationUniform,
  pub bias: ShadowBias,
  pub map_info: ShadowMapAddressInfo,
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq)]
pub struct ShadowBias {
  pub bias: f32,
  pub normal_bias: f32,
}

impl ShadowBias {
  pub fn new(bias: f32, normal_bias: f32) -> Self {
    Self {
      bias,
      normal_bias,
      ..Zeroable::zeroed()
    }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq)]
pub struct ShadowMapAddressInfo {
  pub layer_index: i32,
  /// in pixel unit
  pub size: Vec2<f32>,
  /// in pixel unit
  pub offset: Vec2<f32>,
}

pub trait ShadowOcclusionQuery {
  fn query_shadow_occlusion(
    &self,
    render_position: Node<Vec3<f32>>,
    render_normal: Node<Vec3<f32>>,
    camera_world_position: Node<HighPrecisionTranslation>,
    camera_world_none_translation_mat: Node<Mat4<f32>>,
  ) -> Node<f32>;
}

pub fn create_shadow_depth_sampler_desc(reversed_depth: bool) -> SamplerDescriptor<'static> {
  SamplerDescriptor {
    mag_filter: rendiation_webgpu::FilterMode::Linear,
    min_filter: rendiation_webgpu::FilterMode::Linear,
    mipmap_filter: rendiation_webgpu::FilterMode::Nearest,
    compare: Some(if reversed_depth {
      CompareFunction::Greater
    } else {
      CompareFunction::Less
    }),
    ..Default::default()
  }
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
