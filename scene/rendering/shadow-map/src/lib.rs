use std::task::{Context, Poll};

use reactive::*;
use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_texture_core::*;
use rendiation_texture_packer::pack_2d_to_3d::{
  reactive_pack_2d_to_3d, MultiLayerTexturePackerConfig, PackResult2dWithDepth,
};
use rendiation_webgpu::*;
use rendiation_webgpu_reactive_utils::*;

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
  config: MultiLayerTexturePackerConfig,
  gpu_ctx: &GPUResourceCtx,
) -> (
  BasicShadowMapSystem,
  UniformArrayUpdateContainer<BasicShadowMapInfo>,
) {
  let source_view_proj = inputs.source_view_proj.into_forker();
  let (sys, address) =
    BasicShadowMapSystem::new(config, source_view_proj.clone().into_boxed(), inputs.size);

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
  shadow_map_atlas: Option<GPUTexture>,
  packing: Box<dyn ReactiveCollection<u32, ShadowMapAddressInfo>>,
  atlas_resize: Box<dyn Stream<Item = SizeWithDepth> + Unpin>,
  current_size: Option<SizeWithDepth>,
  source_view_proj: Box<dyn ReactiveCollection<u32, Mat4<f32>>>,
}

impl BasicShadowMapSystem {
  pub fn new(
    config: MultiLayerTexturePackerConfig,
    source_view_proj: Box<dyn ReactiveCollection<u32, Mat4<f32>>>,
    size: Box<dyn ReactiveCollection<u32, Size>>,
  ) -> (Self, Box<dyn ReactiveCollection<u32, ShadowMapAddressInfo>>) {
    let (packing, atlas_resize) = reactive_pack_2d_to_3d(config, size);
    let packing = packing.collective_map(convert_pack_result).into_forker();

    let sys = Self {
      shadow_map_atlas: None,
      current_size: None,
      packing: packing.clone().into_boxed(),
      atlas_resize: Box::new(atlas_resize),
      source_view_proj,
    };
    (sys, packing.into_boxed())
  }

  pub fn update_shadow_maps(
    &mut self,
    cx: &mut Context,
    frame_ctx: &mut FrameCtx,
    scene_content: impl PassContent + Clone,
  ) {
    let _ = self.packing.poll_changes(cx); // incremental detail is useless here
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

    let current_layouts = self.packing.access();
    let view_proj_mats = self.source_view_proj.access();

    // do shadowmap updates
    for (idx, shadow_view) in current_layouts.iter_key_value() {
      let _view_proj = view_proj_mats.access(&idx).unwrap();

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

      // custom dispatcher is not required because we only have depth output.
      let mut pass = pass("shadow-map")
        .with_depth(write_view, clear(1.))
        .render_ctx(frame_ctx);

      let raw_pass = &mut pass.pass.ctx.pass;
      let x = shadow_view.offset.x;
      let y = shadow_view.offset.y;
      let w = shadow_view.size.x;
      let h = shadow_view.size.y;
      raw_pass.set_viewport(x, y, w, h, 0., 1.);

      pass.by(scene_content.clone());
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
