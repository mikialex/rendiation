mod transform_instance;
pub use transform_instance::*;
mod attributes;
pub use attributes::*;

use crate::*;

pub fn map_topology(pt: PrimitiveTopology) -> rendiation_webgpu::PrimitiveTopology {
  match pt {
    PrimitiveTopology::PointList => rendiation_webgpu::PrimitiveTopology::PointList,
    PrimitiveTopology::LineList => rendiation_webgpu::PrimitiveTopology::LineList,
    PrimitiveTopology::LineStrip => rendiation_webgpu::PrimitiveTopology::LineStrip,
    PrimitiveTopology::TriangleList => rendiation_webgpu::PrimitiveTopology::TriangleList,
    PrimitiveTopology::TriangleStrip => rendiation_webgpu::PrimitiveTopology::TriangleStrip,
  }
}
