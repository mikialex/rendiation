mod transform_instance;
pub use transform_instance::*;

use crate::*;
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

pub struct MeshGPUResource {
  att: StorageReadView<AttributesMesh>,
  attributes: AttributeMeshGPUResource,
}

pub enum SceneMeshRenderComponent<'a> {
  Att(AttributesMeshGPU<'a>),
}

impl<'a> MeshDrawcallEmitter for SceneMeshRenderComponent<'a> {
  fn draw_command(&self, group: MeshDrawGroup) -> DrawCommand {
    match self {
      SceneMeshRenderComponent::Att(mesh) => mesh.draw_command(group),
    }
  }
}

impl MeshGPUResource {
  pub fn prepare_render(&self, mesh: &MeshEnum) -> SceneMeshRenderComponent {
    match mesh {
      MeshEnum::AttributesMesh(m) => {
        let id = m.alloc_index().into();
        let mesh = self.att.get(id).unwrap();
        let mesh = AttributesMeshGPU {
          mesh,
          mesh_id: id,
          resource_ctx: &self.attributes,
        };
        SceneMeshRenderComponent::Att(mesh)
      }
      _ => todo!(),
    }
  }
}
