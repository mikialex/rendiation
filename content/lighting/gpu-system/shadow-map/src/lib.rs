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

pub struct BasicShadowMapSystemInputs {
  /// alloc_id => shadow map world
  pub source_world: BoxedDynReactiveQuery<u32, Mat4<f32>>,
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
  inputs: BasicShadowMapSystemInputs,
  config: MultiLayerTexturePackerConfig,
  gpu_ctx: &GPU,
) -> (
  BasicShadowMapSystem,
  UniformArrayUpdateContainer<BasicShadowMapInfo>,
) {
  let source_world = inputs.source_world.into_forker();

  let source_proj = inputs.source_proj.into_forker();

  let source_view_proj = source_world
    .clone()
    .collective_zip(source_proj.clone())
    .collective_map(|(w, p)| p * w.inverse_or_identity())
    .into_boxed();

  let (sys, address) = BasicShadowMapSystem::new(
    config,
    source_world.into_boxed(),
    source_proj.into_boxed(),
    inputs.size,
  );

  let map_info = address
    .into_query_update_uniform_array(std::mem::offset_of!(BasicShadowMapInfo, map_info), gpu_ctx);

  let bias = inputs
    .bias
    .into_query_update_uniform_array(std::mem::offset_of!(BasicShadowMapInfo, bias), gpu_ctx);

  let shadow_camera_view_proj = source_view_proj.into_query_update_uniform_array(
    std::mem::offset_of!(BasicShadowMapInfo, shadow_camera_view_proj),
    gpu_ctx,
  );

  let uniforms = UniformBufferDataView::create_default(&gpu_ctx.device);
  let uniforms = UniformArrayUpdateContainer::<BasicShadowMapInfo>::new(uniforms)
    .with_source(map_info)
    .with_source(shadow_camera_view_proj)
    .with_source(bias);

  (sys, uniforms)
}

pub struct BasicShadowMapSystem {
  shadow_map_atlas: Option<GPUTexture>,
  packing: BoxedDynReactiveQuery<u32, ShadowMapAddressInfo>,
  atlas_resize: Box<dyn Stream<Item = SizeWithDepth> + Unpin>,
  current_size: Option<SizeWithDepth>,
  source_world: BoxedDynReactiveQuery<u32, Mat4<f32>>,
  source_proj: BoxedDynReactiveQuery<u32, Mat4<f32>>,
}

impl BasicShadowMapSystem {
  pub fn new(
    config: MultiLayerTexturePackerConfig,
    source_world: BoxedDynReactiveQuery<u32, Mat4<f32>>,
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
  pub fn update_shadow_maps<'a>(
    &mut self,
    cx: &mut Context,
    frame_ctx: &mut FrameCtx,
    // proj, world
    scene_content: &impl Fn(Mat4<f32>, Mat4<f32>, &mut FrameCtx) -> Box<dyn PassContent + 'a>,
    reversed_depth: bool,
  ) -> GPU2DArrayDepthTextureView {
    let (_, current_layouts) = self.packing.poll_changes(cx); // incremental detail is useless here
    while let Poll::Ready(Some(new_size)) = self.atlas_resize.poll_next_unpin(cx) {
      // if we do shadow cache, we should also do content copy
      self.current_size = Some(new_size);
      self.shadow_map_atlas = None;
    }

    let shadow_map_atlas = self.shadow_map_atlas.get_or_insert_with(|| {
      GPUTexture::create(
        TextureDescriptor {
          label: "shadow-map-atlas".into(),
          size: self.current_size.unwrap().into_gpu_size(),
          mip_level_count: 1,
          sample_count: 1,
          dimension: TextureDimension::D2,
          format: TextureFormat::Depth32Float,
          view_formats: &[],
          usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        },
        &frame_ctx.gpu.device,
      )
    });

    let (_, world) = self.source_world.poll_changes(cx);
    let (_, proj) = self.source_proj.poll_changes(cx);

    for layer in 0..u32::from(self.current_size.unwrap().depth) {
      // clear all
      let write_view: GPU2DTextureView = shadow_map_atlas
        .create_view(TextureViewDescriptor {
          label: Some("shadowmap-clear-view"),
          dimension: Some(TextureViewDimension::D2),
          base_array_layer: layer,
          array_layer_count: Some(1),
          ..Default::default()
        })
        .try_into()
        .unwrap();

      let _ = pass("shadow-map-clear")
        .with_depth(write_view, clear(if reversed_depth { 0. } else { 1. }))
        .render_ctx(frame_ctx);
    }

    // do shadowmap updates
    for (idx, shadow_view) in current_layouts.iter_key_value() {
      let world = world.access(&idx).unwrap();
      let proj = proj.access(&idx).unwrap();

      let write_view: GPU2DTextureView = shadow_map_atlas
        .create_view(TextureViewDescriptor {
          label: Some("shadowmap-write-view"),
          dimension: Some(TextureViewDimension::D2),
          base_array_layer: shadow_view.layer_index as u32,
          array_layer_count: Some(1),
          ..Default::default()
        })
        .try_into()
        .unwrap();

      let mut scene_content = scene_content(proj, world, frame_ctx);

      // todo, consider merge the pass within the same layer
      // custom dispatcher is not required because we only have depth output.
      let mut pass = pass("shadow-map")
        .with_depth(write_view, load())
        .render_ctx(frame_ctx);

      let raw_pass = &mut pass.pass.ctx.pass;
      let x = shadow_view.offset.x;
      let y = shadow_view.offset.y;
      let w = shadow_view.size.x;
      let h = shadow_view.size.y;
      raw_pass.set_viewport(x, y, w, h, 0., 1.);

      pass.by(&mut scene_content);
    }

    shadow_map_atlas
      .create_view(TextureViewDescriptor {
        dimension: TextureViewDimension::D2Array.into(),
        ..Default::default()
      })
      .try_into()
      .unwrap()
  }
}

fn convert_pack_result(r: PackResult2dWithDepth) -> ShadowMapAddressInfo {
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
  pub shadow_camera_view_proj: Mat4<f32>,
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
    world_position: Node<Vec3<f32>>,
    world_normal: Node<Vec3<f32>>,
  ) -> Node<f32>;
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
    ctx.bind_immediate_sampler(&SamplerDescriptor {
      mag_filter: rendiation_webgpu::FilterMode::Linear,
      min_filter: rendiation_webgpu::FilterMode::Linear,
      mipmap_filter: rendiation_webgpu::FilterMode::Nearest,
      compare: Some(if self.reversed_depth {
        CompareFunction::Greater
      } else {
        CompareFunction::Less
      }),
      ..Default::default()
    });
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

#[derive(Clone, Copy)]
pub struct BasicShadowMapInvocation {
  shadow_map_atlas: HandleNode<ShaderDepthTexture2DArray>,
  sampler: HandleNode<ShaderCompareSampler>,
  info: UniformNode<Shader140Array<BasicShadowMapInfo, 8>>,
}

impl BasicShadowMapInvocation {
  pub fn query_shadow_occlusion_by_idx(
    &self,
    world_position: Node<Vec3<f32>>,
    world_normal: Node<Vec3<f32>>,
    shadow_idx: Node<u32>,
  ) -> Node<f32> {
    let shadow_info = self.info.index(shadow_idx).load().expand();

    let bias = shadow_info.bias.expand();

    // apply normal bias
    let world_position = world_position + bias.normal_bias * world_normal;

    let shadow_position = shadow_info.shadow_camera_view_proj * (world_position, val(1.)).into();

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
      inner: self,
      iter: self.info.into_shader_iter(),
    }
  }
}

#[derive(Clone)]
pub struct BasicShadowMapInvocationIter {
  inner: BasicShadowMapInvocation,
  iter: UniformArrayIter<BasicShadowMapInfo, 8>,
}

impl ShaderIterator for BasicShadowMapInvocationIter {
  type Item = BasicShadowMapSingleInvocation;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let (valid, (index, _)) = self.iter.shader_next();

    let item = BasicShadowMapSingleInvocation {
      sys: self.inner,
      index,
    };

    (valid, item)
  }
}

#[derive(Clone, Copy)]
pub struct BasicShadowMapSingleInvocation {
  sys: BasicShadowMapInvocation,
  index: Node<u32>,
}

impl ShadowOcclusionQuery for BasicShadowMapSingleInvocation {
  fn query_shadow_occlusion(
    &self,
    world_position: Node<Vec3<f32>>,
    world_normal: Node<Vec3<f32>>,
  ) -> Node<f32> {
    self
      .sys
      .query_shadow_occlusion_by_idx(world_position, world_normal, self.index)
  }
}

fn sample_shadow_pcf_x36_by_offset(
  map: HandleNode<ShaderDepthTexture2DArray>,
  shadow_position: Node<Vec3<f32>>,
  d_sampler: HandleNode<ShaderCompareSampler>,
  info: ENode<ShadowMapAddressInfo>,
) -> Node<f32> {
  let uv = shadow_position.xy();
  let depth = shadow_position.z();
  let layer = info.layer_index;
  let mut ratio = val(0.0);

  let map_size = map.texture_dimension_2d(None).into_f32();
  let extra_scale = info.size / map_size;

  let uv = uv * extra_scale + info.offset / map_size;

  let s = 2_i32; // we should write a for here?

  for i in -1..=1 {
    for j in -1..=1 {
      let result = map
        .build_compare_sample_call(d_sampler, uv, depth)
        .with_offset((s * i, s * j).into())
        .with_array_index(layer)
        .sample();
      ratio += result;
    }
  }

  ratio / val(9.)
}
