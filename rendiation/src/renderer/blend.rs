use wgpu::{BlendDescriptor, BlendFactor, BlendOperation};

pub struct Blend {
  pub color_blend: wgpu::BlendDescriptor,
  pub alpha_blend: wgpu::BlendDescriptor,
}

impl Blend {
  pub const REPLACE: Self = Self {
    color_blend: BlendDescriptor {
      src_factor: BlendFactor::One,
      dst_factor: BlendFactor::Zero,
      operation: BlendOperation::Add,
    },
    alpha_blend: BlendDescriptor {
      src_factor: BlendFactor::One,
      dst_factor: BlendFactor::Zero,
      operation: BlendOperation::Add,
    },
  };

  pub const NORMAL: Self = Self {
    color_blend: BlendDescriptor {
      src_factor: BlendFactor::SrcAlpha,
      dst_factor: BlendFactor::OneMinusSrcAlpha,
      operation: BlendOperation::Add,
    },
    alpha_blend: BlendDescriptor {
      src_factor: BlendFactor::SrcAlpha,
      dst_factor: BlendFactor::OneMinusSrcAlpha,
      operation: BlendOperation::Add,
    },
  };
}
