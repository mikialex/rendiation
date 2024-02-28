mod transform_instance;
pub use transform_instance::*;

use crate::*;
mod attributes;
pub use attributes::*;
use rendiation_mesh_core::MeshDrawGroup;
use rendiation_webgpu::DrawCommand;

pub trait MeshDrawcallEmitter {
  fn draw_command(&self, group: MeshDrawGroup) -> DrawCommand;
}

pub struct MeshGPUResource {
  att: StorageReadView<AttributesMesh>,
  attributes: AttributeMeshGPUResource,
}

pub enum SceneMeshRenderComponent<'a> {
  Att(AttributesMeshGPU<'a>),
  Instance(TransformInstanceGPU<'a>),
}

impl<'a> ShaderHashProvider for SceneMeshRenderComponent<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    std::mem::discriminant(self).hash(hasher);
    match self {
      Self::Att(m) => m.hash_pipeline(hasher),
      Self::Instance(m) => m.hash_pipeline(hasher),
    }
  }
}
impl<'a> ShaderHashProviderAny for SceneMeshRenderComponent<'a> {
  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    struct Marker;
    Marker.type_id().hash(hasher);
    self.hash_pipeline(hasher);
  }
}
impl<'a> GraphicsShaderProvider for SceneMeshRenderComponent<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    match self {
      Self::Att(m) => m.build(builder),
      Self::Instance(m) => m.build(builder),
    }
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    match self {
      Self::Att(m) => m.post_build(builder),
      Self::Instance(m) => m.post_build(builder),
    }
  }
}
impl<'a> ShaderPassBuilder for SceneMeshRenderComponent<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      Self::Att(m) => m.setup_pass(ctx),
      Self::Instance(m) => m.setup_pass(ctx),
    }
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      Self::Att(m) => m.post_setup_pass(ctx),
      Self::Instance(m) => m.post_setup_pass(ctx),
    }
  }
}

impl<'a> MeshDrawcallEmitter for SceneMeshRenderComponent<'a> {
  fn draw_command(&self, group: MeshDrawGroup) -> DrawCommand {
    match self {
      SceneMeshRenderComponent::Att(m) => m.draw_command(group),
      SceneMeshRenderComponent::Instance(m) => m.draw_command(group),
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
