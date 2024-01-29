mod transform_instance;
pub use transform_instance::*;
mod attributes;
pub use attributes::*;
use rendiation_mesh_core::{AttributeAccessor, AttributeIndexFormat, MeshDrawGroup};
use rendiation_webgpu::{DrawCommand, GPUBufferResourceView, IndexFormat};

pub fn map_topology(
  pt: rendiation_mesh_core::PrimitiveTopology,
) -> rendiation_webgpu::PrimitiveTopology {
  use rendiation_mesh_core::PrimitiveTopology as Enum;
  use rendiation_webgpu::PrimitiveTopology as GPUEnum;
  match pt {
    Enum::PointList => GPUEnum::PointList,
    Enum::LineList => GPUEnum::LineList,
    Enum::LineStrip => GPUEnum::LineStrip,
    Enum::TriangleList => GPUEnum::TriangleList,
    Enum::TriangleStrip => GPUEnum::TriangleStrip,
  }
}

pub fn map_index(index: AttributeIndexFormat) -> IndexFormat {
  match index {
    AttributeIndexFormat::Uint16 => IndexFormat::Uint16,
    AttributeIndexFormat::Uint32 => IndexFormat::Uint32,
  }
}

pub trait MeshDrawcallEmitter {
  fn draw_command(&self, group: MeshDrawGroup) -> DrawCommand;
}

pub struct MeshVertexBufferManager {}

impl MeshVertexBufferManager {
  pub fn get_gpu_vertex(&self, acc: &AttributeAccessor) -> &GPUBufferResourceView {
    todo!()
  }
}
pub struct MeshIndexBufferManager {}

impl MeshIndexBufferManager {
  pub fn get_gpu_index(&self, acc: &AttributeAccessor) -> &GPUBufferResourceView {
    todo!()
  }
}
