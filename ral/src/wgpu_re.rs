pub use wgpu::{
  BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite, CullMode,
  DepthStencilStateDescriptor, FrontFace, PrimitiveTopology, RasterizationStateDescriptor,
  ShaderStage, StencilStateDescriptor, TextureFormat, VertexBufferDescriptor, VertexFormat,
  VertexStateDescriptor,
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TargetStates {
  pub color_states: Vec<ColorStateDescriptor>,
  pub depth_state: Option<DepthStencilStateDescriptor>,
}
