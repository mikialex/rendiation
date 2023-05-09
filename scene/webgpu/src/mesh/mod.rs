use rendiation_renderable_mesh::mesh::IntersectAbleGroupedMesh;

pub mod typed;
pub use typed::*;
pub mod transform_instance;
pub use transform_instance::*;
pub mod free_attributes;
pub use free_attributes::*;

use crate::*;

pub type ReactiveMeshGPUOf<T> = <T as WebGPUMesh>::ReactiveGPU;

pub trait WebGPUSceneMesh: Any + Send + Sync {
  fn create_scene_reactive_gpu(&self, ctx: &ShareBindableResourceCtx) -> Option<MeshGPUInstance>;

  fn topology(&self) -> webgpu::PrimitiveTopology;
  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand;

  // the reason we use CPS style is for supporting refcell
  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh));
}
define_dyn_trait_downcaster_static!(WebGPUSceneMesh);
pub fn register_webgpu_mesh_features<T>()
where
  T: AsRef<dyn WebGPUSceneMesh> + AsMut<dyn WebGPUSceneMesh> + 'static,
{
  get_dyn_trait_downcaster_static!(WebGPUSceneMesh).register::<T>()
}

impl WebGPUSceneMesh for SceneMeshType {
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
        .downcast_ref(m.as_ref())?
        .create_scene_reactive_gpu(ctx)?,
      _ => return None,
    }
    .into()
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    match self {
      SceneMeshType::AttributesMesh(m) => m.topology(),
      SceneMeshType::TransformInstanced(m) => m.topology(),
      SceneMeshType::Foreign(m) => {
        if let Some(m) = get_dyn_trait_downcaster_static!(WebGPUSceneMesh).downcast_ref(m.as_ref())
        {
          m.topology()
        } else {
          webgpu::PrimitiveTopology::TriangleList
        }
      }
      _ => webgpu::PrimitiveTopology::TriangleList,
    }
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    match self {
      SceneMeshType::AttributesMesh(m) => m.draw_impl(group),
      SceneMeshType::TransformInstanced(m) => m.draw_impl(group),
      SceneMeshType::Foreign(m) => {
        if let Some(mesh) =
          get_dyn_trait_downcaster_static!(WebGPUSceneMesh).downcast_ref(m.as_ref())
        {
          mesh.draw_impl(group)
        } else {
          DrawCommand::Skip
        }
      }
      _ => DrawCommand::Skip,
    }
  }
  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    match self {
      SceneMeshType::AttributesMesh(_) => {}
      SceneMeshType::TransformInstanced(m) => m.try_pick(f),
      SceneMeshType::Foreign(m) => {
        if let Some(mesh) =
          get_dyn_trait_downcaster_static!(WebGPUSceneMesh).downcast_ref(m.as_ref())
        {
          mesh.try_pick(f)
        }
      }
      _ => {}
    }
  }
}

impl<T: WebGPUMesh> WebGPUSceneMesh for SceneItemRef<T> {
  fn create_scene_reactive_gpu(&self, ctx: &ShareBindableResourceCtx) -> Option<MeshGPUInstance> {
    let instance = T::create_reactive_gpu(self, ctx);
    MeshGPUInstance::Foreign(Box::new(instance) as Box<dyn ReactiveRenderComponentSource>).into()
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.read().topology()
  }

  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    let inner = self.read();
    inner.try_pick(f);
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    self.read().draw_impl(group)
  }
}
impl<T: WebGPUMesh> AsRef<dyn WebGPUSceneMesh> for SceneItemRef<T> {
  fn as_ref(&self) -> &(dyn WebGPUSceneMesh + 'static) {
    self
  }
}
impl<T: WebGPUMesh> AsMut<dyn WebGPUSceneMesh> for SceneItemRef<T> {
  fn as_mut(&mut self) -> &mut (dyn WebGPUSceneMesh + 'static) {
    self
  }
}

impl<T: WebGPUSceneMesh> MeshDrawcallEmitter for T {
  fn draw(&self, ctx: &mut webgpu::GPURenderPassCtx, group: MeshDrawGroup) {
    ctx.pass.draw_by_command(self.draw_impl(group))
  }
}

pub trait WebGPUMesh: Any + Send + Sync + Incremental {
  type ReactiveGPU: ReactiveRenderComponentSource;
  fn create_reactive_gpu(
    source: &SceneItemRef<Self>,
    ctx: &ShareBindableResourceCtx,
  ) -> Self::ReactiveGPU;

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand;

  fn topology(&self) -> webgpu::PrimitiveTopology;

  fn try_pick(&self, _f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {}
}

#[pin_project::pin_project(project = MeshGPUInstanceProj)]
pub enum MeshGPUInstance {
  Attributes(ReactiveMeshGPUOf<AttributesMesh>),
  TransformInstanced(ReactiveMeshGPUOf<TransformInstancedSceneMesh>),
  Foreign(Box<dyn ReactiveRenderComponentSource>),
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
      Self::Attributes(m) => Box::pin(m.as_ref().create_render_component_delta_stream())
        as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>,
      Self::TransformInstanced(m) => Box::pin(m.as_ref().create_render_component_delta_stream())
        as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>,
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
        .hash_pipeline_and_with_type_id(hasher),
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

impl ShaderGraphProvider for MeshGPUInstance {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    match self {
      Self::Attributes(m) => m.as_reactive_component().build(builder),
      Self::TransformInstanced(m) => m.as_reactive_component().build(builder),
      Self::Foreign(m) => m.as_reactive_component().build(builder),
    }
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
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
    mesh: &SceneMeshType,
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
