mod transform_instance;
pub use transform_instance::*;
mod attributes;
pub use attributes::*;
mod utils;
pub use utils::*;

use crate::*;

pub type ReactiveMeshGPUOf<T> = <T as WebGPUMesh>::ReactiveGPU;

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

pub trait WebGPUSceneMesh: Any + Send + Sync {
  fn create_scene_reactive_gpu(&self, ctx: &ShareBindableResourceCtx) -> Option<MeshGPUInstance>;
}
define_dyn_trait_downcaster_static!(WebGPUSceneMesh);
pub fn register_webgpu_mesh_features<T>()
where
  T: AsRef<dyn WebGPUSceneMesh> + AsMut<dyn WebGPUSceneMesh> + 'static,
{
  get_dyn_trait_downcaster_static!(WebGPUSceneMesh).register::<T>()
}

impl WebGPUSceneMesh for MeshEnum {
  fn create_scene_reactive_gpu(&self, ctx: &ShareBindableResourceCtx) -> Option<MeshGPUInstance> {
    match self {
      Self::AttributesMesh(m) => {
        let instance = AttributesMesh::create_reactive_gpu(m, ctx);
        MeshGPUInstance::Attributes(instance)
      }
      Self::TransformInstanced(m) => {
        let instance = TransformInstancedSceneMesh::create_reactive_gpu(m, ctx);
        MeshGPUInstance::TransformInstanced(instance)
      }
      Self::Foreign(m) => get_dyn_trait_downcaster_static!(WebGPUSceneMesh)
        .downcast_ref(m.as_ref().as_any())?
        .create_scene_reactive_gpu(ctx)?,
    }
    .into()
  }
}

impl<T: WebGPUMesh> WebGPUSceneMesh for IncrementalSignalPtr<T> {
  fn create_scene_reactive_gpu(&self, ctx: &ShareBindableResourceCtx) -> Option<MeshGPUInstance> {
    let instance = T::create_reactive_gpu(self, ctx);
    MeshGPUInstance::Foreign(Box::new(instance) as Box<dyn ReactiveMeshGPUSource>).into()
  }
}
impl<T: WebGPUMesh> AsRef<dyn WebGPUSceneMesh> for IncrementalSignalPtr<T> {
  fn as_ref(&self) -> &(dyn WebGPUSceneMesh + 'static) {
    self
  }
}
impl<T: WebGPUMesh> AsMut<dyn WebGPUSceneMesh> for IncrementalSignalPtr<T> {
  fn as_mut(&mut self) -> &mut (dyn WebGPUSceneMesh + 'static) {
    self
  }
}

pub trait WebGPUMesh: Any + Send + Sync + IncrementalBase {
  type ReactiveGPU: ReactiveMeshGPUSource;
  fn create_reactive_gpu(
    source: &IncrementalSignalPtr<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU;
}

#[pin_project::pin_project(project = MeshGPUInstanceProj)]
pub enum MeshGPUInstance {
  Attributes(ReactiveMeshGPUOf<AttributesMesh>),
  TransformInstanced(ReactiveMeshGPUOf<TransformInstancedSceneMesh>),
  Foreign(Box<dyn ReactiveMeshGPUSource>),
}

impl MeshGPUInstance {
  pub fn get_bindless(&self) -> Option<MeshSystemMeshHandle> {
    match self {
      MeshGPUInstance::Attributes(mesh) => {
        let mesh: &RenderComponentCell<MaybeBindlessMesh<AttributesMeshGPU>> = mesh.inner.as_ref();
        match &mesh.inner {
          MaybeBindlessMesh::Bindless(m) => Some(m.mesh_handle()),
          _ => None,
        }
      }
      _ => None,
    }
  }
}

pub trait ReactiveMeshGPUSource: ReactiveRenderComponentSource + MeshDrawcallEmitter {}
impl<T: ReactiveRenderComponentSource + MeshDrawcallEmitter> ReactiveMeshGPUSource for T {}

pub trait MeshDrawcallEmitter {
  fn draw_command(&self, group: MeshDrawGroup) -> DrawCommand;
}

impl MeshDrawcallEmitter for MeshGPUInstance {
  fn draw_command(&self, group: MeshDrawGroup) -> DrawCommand {
    match self {
      Self::Attributes(m) => m.draw_command(group),
      Self::TransformInstanced(m) => m.draw_command(group),
      Self::Foreign(m) => m.draw_command(group),
    }
  }
}

impl Stream for MeshGPUInstance {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    match self.project() {
      MeshGPUInstanceProj::Attributes(m) => m.poll_next_unpin(cx),
      MeshGPUInstanceProj::TransformInstanced(m) => m.poll_next_unpin(cx),
      MeshGPUInstanceProj::Foreign(m) => m.poll_next_unpin(cx),
    }
  }
}

impl ReactiveRenderComponent for MeshGPUInstance {
  fn create_render_component_delta_stream(
    &self,
  ) -> Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>> {
    match self {
      Self::Attributes(m) => Box::pin(m.inner.as_ref().create_render_component_delta_stream())
        as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>,
      Self::TransformInstanced(m) => {
        Box::pin(m.inner.as_ref().create_render_component_delta_stream())
          as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>
      }
      Self::Foreign(m) => m
        .as_reactive_component()
        .create_render_component_delta_stream(),
    }
  }
}

impl ShaderHashProvider for MeshGPUInstance {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    std::mem::discriminant(self).hash(hasher);
    match self {
      Self::Attributes(m) => m.as_reactive_component().hash_pipeline(hasher),
      Self::TransformInstanced(m) => m.as_reactive_component().hash_pipeline(hasher),
      Self::Foreign(m) => m
        .as_reactive_component()
        .hash_pipeline_with_type_info(hasher),
    }
  }
}

impl ShaderPassBuilder for MeshGPUInstance {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      Self::Attributes(m) => m.as_reactive_component().setup_pass(ctx),
      Self::TransformInstanced(m) => m.as_reactive_component().setup_pass(ctx),
      Self::Foreign(m) => m.as_reactive_component().setup_pass(ctx),
    }
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      Self::Attributes(m) => m.as_reactive_component().post_setup_pass(ctx),
      Self::TransformInstanced(m) => m.as_reactive_component().post_setup_pass(ctx),
      Self::Foreign(m) => m.as_reactive_component().post_setup_pass(ctx),
    }
  }
}

impl GraphicsShaderProvider for MeshGPUInstance {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    match self {
      Self::Attributes(m) => m.as_reactive_component().build(builder),
      Self::TransformInstanced(m) => m.as_reactive_component().build(builder),
      Self::Foreign(m) => m.as_reactive_component().build(builder),
    }
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    match self {
      Self::Attributes(m) => m.as_reactive_component().post_build(builder),
      Self::TransformInstanced(m) => m.as_reactive_component().post_build(builder),
      Self::Foreign(m) => m.as_reactive_component().post_build(builder),
    }
  }
}

pub type ReactiveMeshRenderComponentDeltaSource = impl Stream<Item = RenderComponentDeltaFlag>;

impl GPUModelResourceCtx {
  pub fn get_or_create_reactive_mesh_render_component_delta_source(
    &self,
    mesh: &MeshEnum,
  ) -> Option<ReactiveMeshRenderComponentDeltaSource> {
    self
      .meshes
      .write()
      .unwrap()
      .get_or_insert_with(mesh.guid()?, || {
        mesh.create_scene_reactive_gpu(&self.shared).unwrap()
      })
      .create_render_component_delta_stream()
      .into()
  }
}
