mod transform_instance;
pub use transform_instance::*;
mod attributes;
pub use attributes::*;

use crate::*;

pub fn map_topology(pt: PrimitiveTopology) -> rendiation_webgpu::PrimitiveTopology {
  match pt {
    Enum::PointList => GPUEnum::PointList,
    Enum::LineList => GPUEnum::LineList,
    Enum::LineStrip => GPUEnum::LineStrip,
    Enum::TriangleList => GPUEnum::TriangleList,
    Enum::TriangleStrip => GPUEnum::TriangleStrip,
  }
}
