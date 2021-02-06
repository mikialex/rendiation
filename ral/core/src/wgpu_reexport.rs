pub use wgpu_types::{
  AddressMode, BindGroupLayoutEntry, BindingType, BlendFactor, BlendOperation, BlendState,
  BufferBindingType, ColorTargetState, ColorWrite, CullMode, DepthStencilState, FrontFace,
  IndexFormat, InputStepMode, PrimitiveState, PrimitiveTopology, ShaderLocation, ShaderStage,
  StencilState, TextureFormat, TextureSampleType, TextureViewDimension, VertexAttribute,
  VertexFormat,
};

/// Describes how the vertex buffer is interpreted.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct VertexBufferLayout<'a> {
  /// The stride, in bytes, between elements of this buffer.
  pub array_stride: u64,
  /// How often this vertex buffer is "stepped" forward.
  pub step_mode: InputStepMode,
  /// The list of attributes which comprise a single vertex.
  pub attributes: &'a [VertexAttribute],
}

/// Describes a [`BindGroupLayout`].
#[derive(Clone, Debug)]
pub struct BindGroupLayoutDescriptor<'a> {
  /// Debug label of the bind group layout. This will show up in graphics debuggers for easy identification.
  pub label: Option<&'a str>,

  /// Array of entries in this BindGroupLayout
  pub entries: &'a [BindGroupLayoutEntry],
}
