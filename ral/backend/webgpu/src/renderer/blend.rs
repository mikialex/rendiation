use wgpu::{BlendFactor, BlendOperation, BlendState};

pub struct Blend {
  pub color_blend: wgpu::BlendState,
  pub alpha_blend: wgpu::BlendState,
}

impl Blend {
  pub const REPLACE: Self = Self {
    color_blend: BlendState {
      src_factor: BlendFactor::One,
      dst_factor: BlendFactor::Zero,
      operation: BlendOperation::Add,
    },
    alpha_blend: BlendState {
      src_factor: BlendFactor::One,
      dst_factor: BlendFactor::Zero,
      operation: BlendOperation::Add,
    },
  };

  pub const NORMAL: Self = Self {
    color_blend: BlendState {
      src_factor: BlendFactor::SrcAlpha,
      dst_factor: BlendFactor::OneMinusSrcAlpha,
      operation: BlendOperation::Add,
    },
    alpha_blend: BlendState {
      src_factor: BlendFactor::SrcAlpha,
      dst_factor: BlendFactor::OneMinusSrcAlpha,
      operation: BlendOperation::Add,
    },
  };
}
