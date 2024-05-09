#![feature(offset_of)]

use std::task::{Context, Poll};

use reactive::*;
use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_texture_core::*;
use rendiation_texture_packer::{
  growable::GrowablePacker,
  pack_2d_to_2d::pack_impl::etagere_wrap::EtagerePacker,
  pack_2d_to_3d::{MultiLayerTexturePacker, PackResult2dWithDepth},
};
use rendiation_webgpu::*;
use rendiation_webgpu_reactive_utils::*;

mod packer;
use packer::*;

pub struct BasicShadowMapSystemInputs {
  ///  alloc_id => shadow camera vp
  pub source_view_proj: Box<dyn ReactiveCollection<u32, Mat4<f32>>>,
  /// alloc_id => shadow map resolution
  pub size: Box<dyn ReactiveCollection<u32, Size>>,
  /// alloc_id => shadow map bias
  pub bias: Box<dyn ReactiveCollection<u32, ShadowBias>>,
  /// alloc_id => enabled
  pub enabled: Box<dyn ReactiveCollection<u32, bool>>,
}

pub fn basic_shadow_map_uniform(
  inputs: BasicShadowMapSystemInputs,
  config: ShadowSizeConfig,
  gpu_ctx: &GPUResourceCtx,
) -> (
  BasicShadowMapSystem,
  UniformArrayUpdateContainer<BasicShadowMapInfo>,
) {
  let source_view_proj = inputs.source_view_proj.into_forker();
  let (sys, address) = BasicShadowMapSystem::new(
    gpu_ctx,
    config,
    source_view_proj.clone().into_boxed(),
    inputs.size,
  );

  let map_info = address.into_uniform_array_collection_update(
    std::mem::offset_of!(BasicShadowMapInfo, map_info),
    gpu_ctx,
  );

  let bias = inputs
    .bias
    .into_uniform_array_collection_update(std::mem::offset_of!(BasicShadowMapInfo, bias), gpu_ctx);

  let shadow_camera_view_proj = source_view_proj.into_uniform_array_collection_update(
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
  shadow_map_gpu: GPUTexture,
  packing: Box<dyn ReactiveCollection<u32, ShadowMapAddressInfo>>,
  atlas_resize: Box<dyn Stream<Item = SizeWithDepth> + Unpin>,
  source_view_proj: Box<dyn ReactiveCollection<u32, Mat4<f32>>>,
}

#[derive(Clone, Copy)]
pub struct ShadowSizeConfig {
  pub init_size: SizeWithDepth,
  pub max_size: SizeWithDepth,
}

impl ShadowSizeConfig {
  pub fn make_sure_valid(&mut self) {
    self.max_size.depth = self.max_size.depth.max(self.init_size.depth);
    self.max_size.size.width = self.max_size.size.width.max(self.init_size.size.width);
    self.max_size.size.height = self.max_size.size.height.max(self.init_size.size.height);
  }
}

impl BasicShadowMapSystem {
  pub fn new(
    gpu: &GPUResourceCtx,
    config: ShadowSizeConfig,
    source_view_proj: Box<dyn ReactiveCollection<u32, Mat4<f32>>>,
    size: Box<dyn ReactiveCollection<u32, Size>>,
  ) -> (Self, Box<dyn ReactiveCollection<u32, ShadowMapAddressInfo>>) {
    let (packing, atlas_resize) = reactive_pack_2d_to_3d(config, size);
    let packing = packing.collective_map(convert_pack_result).into_forker();

    let shadow_map_gpu = GPUTexture::create(
      TextureDescriptor {
        label: "shadow-maps".into(),
        size: config.init_size.into_gpu_size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        view_formats: &[],
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
      },
      &gpu.device,
    );

    let sys = Self {
      shadow_map_gpu,
      packing: packing.clone().into_boxed(),
      atlas_resize: Box::new(atlas_resize),
      source_view_proj,
    };
    (sys, packing.into_boxed())
  }

  pub fn update_shadow_maps(
    &mut self,
    cx: &mut Context,
    // frame_ctx: FrameCtx,
    // scene: impl PassContent,
  ) {
    let _ = self.packing.poll_changes(cx); // incremental detail is useless here
    if let Poll::Ready(Some(_new_size)) = self.atlas_resize.poll_next_unpin(cx) {
      // do resize, if we do shadow cache, we should also do map copy
    }

    let current_layouts = self.packing.access();
    let view_proj_mats = self.source_view_proj.access();

    // do shadowmap updates
    for (idx, shadow_view) in current_layouts.iter_key_value() {
      let _view_proj = view_proj_mats.access(&idx).unwrap();

      let _write_view: GPU2DTextureView = self
        .shadow_map_gpu
        .create_view(TextureViewDescriptor {
          label: Some("shadowmap-write-view"),
          dimension: Some(TextureViewDimension::D2),
          base_array_layer: shadow_view.layer_index as u32,
          array_layer_count: Some(1),
          ..Default::default()
        })
        .try_into()
        .unwrap();
      // let viewport = todo
    }
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
  pub size: Vec2<f32>,
  pub offset: Vec2<f32>,
}
