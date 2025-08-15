use std::mem::offset_of;

use reactive::*;
use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_texture_core::*;
pub use rendiation_texture_packer::pack_2d_to_3d::{
  MultiLayerTexturePackerConfig, PackResult2dWithDepth,
};
use rendiation_webgpu::*;
use rendiation_webgpu_reactive_utils::*;

mod basic;
pub use basic::*;

mod cascade;
pub use cascade::*;

mod map_utils;
use map_utils::*;

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
