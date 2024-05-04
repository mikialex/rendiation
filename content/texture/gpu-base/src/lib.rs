#![feature(specialization)]

use std::any::Any;
use std::hash::Hash;

use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_texture_core::*;
use rendiation_webgpu::*;

mod mipmap_gen_2d;
pub use mipmap_gen_2d::*;

pub trait SamplerConvertExt<'a> {
  fn into_gpu(self) -> SamplerDescriptor<'a>;
}

impl<'a> SamplerConvertExt<'a> for rendiation_texture_core::TextureSampler {
  fn into_gpu(self) -> SamplerDescriptor<'a> {
    SamplerDescriptor {
      label: None,
      address_mode_u: convert_wrap(self.address_mode_u),
      address_mode_v: convert_wrap(self.address_mode_v),
      address_mode_w: convert_wrap(self.address_mode_w),
      mag_filter: convert_filter(self.mag_filter),
      min_filter: convert_filter(self.min_filter),
      mipmap_filter: convert_filter(self.mipmap_filter),
      ..Default::default()
    }
  }
}

fn convert_wrap(mode: rendiation_texture_core::AddressMode) -> rendiation_webgpu::AddressMode {
  match mode {
    rendiation_texture_core::AddressMode::ClampToEdge => {
      rendiation_webgpu::AddressMode::ClampToEdge
    }
    rendiation_texture_core::AddressMode::Repeat => rendiation_webgpu::AddressMode::Repeat,
    rendiation_texture_core::AddressMode::MirrorRepeat => {
      rendiation_webgpu::AddressMode::MirrorRepeat
    }
  }
}

fn convert_filter(mode: rendiation_texture_core::FilterMode) -> rendiation_webgpu::FilterMode {
  match mode {
    rendiation_texture_core::FilterMode::Nearest => rendiation_webgpu::FilterMode::Nearest,
    rendiation_texture_core::FilterMode::Linear => rendiation_webgpu::FilterMode::Linear,
  }
}

/// impl foreign WebGPU2DTextureSource for foreign GPUBufferImage
pub struct GPUBufferImageForeignImpl<'a> {
  pub inner: &'a GPUBufferImage,
}

impl<'a> WebGPU2DTextureSource for GPUBufferImageForeignImpl<'a> {
  fn format(&self) -> TextureFormat {
    self.inner.format
  }

  fn as_bytes(&self) -> &[u8] {
    &self.inner.data
  }

  fn size(&self) -> Size {
    self.inner.size
  }
}
