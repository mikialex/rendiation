pub use wgpu_types::{
  AddressMode, BindGroupLayoutEntry, BindingType, BlendDescriptor, BlendFactor, BlendOperation,
  ColorStateDescriptor, ColorWrite, CullMode, DepthStencilStateDescriptor, FrontFace, IndexFormat,
  InputStepMode, PrimitiveTopology, RasterizationStateDescriptor, ShaderLocation, ShaderStage,
  StencilStateDescriptor, TextureFormat, VertexAttributeDescriptor, VertexFormat,
};

/// Describes how the vertex buffer is interpreted.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct VertexBufferDescriptor<'a> {
  /// The stride, in bytes, between elements of this buffer.
  pub stride: u64,
  /// How often this vertex buffer is "stepped" forward.
  pub step_mode: InputStepMode,
  /// The list of attributes which comprise a single vertex.
  pub attributes: &'a [VertexAttributeDescriptor],
}

/// Describes vertex input state for a render pipeline.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct VertexStateDescriptor<'a> {
  /// The format of any index buffers used with this pipeline.
  pub index_format: IndexFormat,
  /// The format of any vertex buffers used with this pipeline.
  pub vertex_buffers: &'a [VertexBufferDescriptor<'a>],
}

/// Describes a [`BindGroupLayout`].
#[derive(Clone, Debug)]
pub struct BindGroupLayoutDescriptor<'a> {
  /// Debug label of the bind group layout. This will show up in graphics debuggers for easy identification.
  pub label: Option<&'a str>,

  /// Array of entries in this BindGroupLayout
  pub entries: &'a [BindGroupLayoutEntry],
}
