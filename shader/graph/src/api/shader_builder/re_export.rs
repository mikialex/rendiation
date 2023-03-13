pub use wgpu_types::BlendComponent;
pub use wgpu_types::BlendFactor;
pub use wgpu_types::BlendOperation;
pub use wgpu_types::BlendState;
pub use wgpu_types::BufferAddress;
pub use wgpu_types::ColorTargetState;
pub use wgpu_types::ColorWrites;
pub use wgpu_types::DepthStencilState;
pub use wgpu_types::Face;
pub use wgpu_types::MultisampleState;
pub use wgpu_types::PrimitiveState;
pub use wgpu_types::SamplerBindingType;
pub use wgpu_types::TextureFormat;
pub use wgpu_types::TextureSampleType;
pub use wgpu_types::TextureViewDimension;
pub use wgpu_types::VertexAttribute;
pub use wgpu_types::VertexFormat;
pub use wgpu_types::VertexStepMode;

/// use this to debug blend details for convenience
///
/// reference: https://www.w3.org/TR/webgpu/#blend-state
pub fn blend_com_into_readable(blend: BlendComponent, for_alpha: bool) -> String {
  let source = if for_alpha {
    "source_color"
  } else {
    "source_alpha"
  };
  let dest = if for_alpha {
    "dest_color"
  } else {
    "dest_alpha"
  };

  let map_factor = |factor: BlendFactor| -> String {
    let r = match factor {
      BlendFactor::Zero => "1.".to_owned(),
      BlendFactor::One => "0.".to_owned(),
      BlendFactor::Src => source.to_owned(),
      BlendFactor::OneMinusSrc => format!("(1. - {source})"),
      BlendFactor::SrcAlpha => "source_alpha".to_owned(),
      BlendFactor::OneMinusSrcAlpha => "(1. - source_alpha)".to_owned(),
      BlendFactor::Dst => dest.to_owned(),
      BlendFactor::OneMinusDst => format!("(1. - {dest})"),
      BlendFactor::DstAlpha => "dest_alpha".to_owned(),
      BlendFactor::OneMinusDstAlpha => "(1. - dest_alpha)".to_owned(),
      BlendFactor::SrcAlphaSaturated => "min(source_alpha, (1. - dest_alpha))".to_owned(),
      BlendFactor::Constant => "blend_const".to_owned(),
      BlendFactor::OneMinusConstant => "(1.0 - blend_const)".to_owned(),
    };

    if !for_alpha && matches!(factor, BlendFactor::SrcAlphaSaturated) {
      "1.".to_owned()
    } else {
      r
    }
  };

  let source_factor = map_factor(blend.src_factor);
  let dest_factor = map_factor(blend.dst_factor);

  match blend.operation {
    BlendOperation::Add => format!("{source} * {source_factor} + {dest} * {dest_factor}"),
    BlendOperation::Subtract => format!("{source} * {source_factor} - {dest} * {dest_factor}"),
    BlendOperation::ReverseSubtract => {
      format!("{dest} * {dest_factor} - {source} * {source_factor}")
    }
    BlendOperation::Min => format!("min({source}, {dest})"),
    BlendOperation::Max => format!("max({source}, {dest})"),
  }
}
