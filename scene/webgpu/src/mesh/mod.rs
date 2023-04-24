use rendiation_renderable_mesh::mesh::IntersectAbleGroupedMesh;

pub mod fatline;
pub use fatline::*;
pub mod typed;
pub use typed::*;
pub mod transform_instance;
pub use transform_instance::*;
pub mod free_attributes;
pub use free_attributes::*;

use crate::*;

pub type ReactiveMeshGPUOf<T> = <T as WebGPUMesh>::ReactiveGPU;

pub trait WebGPUSceneMesh: Any + Send + Sync {
  fn id(&self) -> Option<usize>;
  fn create_scene_reactive_gpu(&self, ctx: &ShareBindableResourceCtx) -> Option<MeshGPUInstance>;

  fn topology(&self) -> webgpu::PrimitiveTopology;
  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand;

  // the reason we use CPS style is for supporting refcell
  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh));
}

impl WebGPUSceneMesh for SceneMeshType {
  fn id(&self) -> Option<usize> {
    match self {
      Self::AttributesMesh(m) => m.id(),
      Self::Foreign(m) => {
        return if let Some(m) = m.downcast_ref::<Box<dyn WebGPUSceneMesh>>() {
          m.id()
        } else {
          None
        }
      }
      _ => return None,
    }
    .into()
  }
  fn create_scene_reactive_gpu(&self, ctx: &ShareBindableResourceCtx) -> Option<MeshGPUInstance> {
    match self {
      Self::AttributesMesh(m) => {
        let instance = AttributesMesh::create_reactive_gpu(m, ctx);
        MeshGPUInstance::Attributes(instance)
      }
      Self::Foreign(m) => {
        return if let Some(m) = m.downcast_ref::<Box<dyn WebGPUSceneMesh>>() {
          m.create_scene_reactive_gpu(ctx)
        } else {
          None
        }
      }
      _ => return None,
    }
    .into()
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    match self {
      SceneMeshType::AttributesMesh(m) => WebGPUSceneMesh::topology(m),
      SceneMeshType::Foreign(mesh) => {
        if let Some(mesh) = mesh.downcast_ref::<Box<dyn WebGPUSceneMesh>>() {
          mesh.topology()
        } else {
          webgpu::PrimitiveTopology::TriangleList
        }
      }
      _ => webgpu::PrimitiveTopology::TriangleList,
    }
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    match self {
      SceneMeshType::AttributesMesh(m) => WebGPUSceneMesh::draw_impl(m, group),
      SceneMeshType::Foreign(mesh) => {
        if let Some(mesh) = mesh.downcast_ref::<Box<dyn WebGPUSceneMesh>>() {
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
      SceneMeshType::Foreign(mesh) => {
        if let Some(mesh) = mesh.downcast_ref::<Box<dyn WebGPUSceneMesh>>() {
          mesh.try_pick(f)
        }
      }
      _ => {}
    }
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

impl<T: WebGPUMesh> WebGPUSceneMesh for SceneItemRef<T> {
  fn id(&self) -> Option<usize> {
    self.id().into()
  }
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

// impl<T> WebGPUMesh for SceneItemRef<T>
// where
//   T: WebGPUMesh,
// {
//   type GPU = T::GPU;

//   fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &GPU, res: &mut AnyMap) {
//     self.read().update(gpu_mesh, gpu, res);
//   }

//   fn create(&self, gpu: &GPU, res: &mut AnyMap) -> Self::GPU {
//     self.read().create(gpu, res)
//   }

//   fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
//     self.read().draw_impl(group)
//   }

//   fn topology(&self) -> webgpu::PrimitiveTopology {
//     self.read().topology()
//   }

//   fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
//     self.read().try_pick(f)
//   }
// }

#[pin_project::pin_project(project = MeshGPUInstanceProj)]
pub enum MeshGPUInstance {
  Attributes(ReactiveMeshGPUOf<AttributesMesh>),
  Foreign(Box<dyn ReactiveRenderComponentSource>),
}

impl Stream for MeshGPUInstance {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    match self.project() {
      MeshGPUInstanceProj::Attributes(m) => m.poll_next_unpin(cx),
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
      Self::Foreign(m) => m
        .as_reactive_component()
        .create_render_component_delta_stream(),
    }
  }
}

impl ShaderHashProvider for MeshGPUInstance {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    match self {
      Self::Attributes(m) => m.as_reactive_component().hash_pipeline(hasher),
      Self::Foreign(m) => m.as_reactive_component().hash_pipeline(hasher),
    }
  }
}

impl ShaderPassBuilder for MeshGPUInstance {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      Self::Attributes(m) => m.as_reactive_component().setup_pass(ctx),
      Self::Foreign(m) => m.as_reactive_component().setup_pass(ctx),
    }
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      Self::Attributes(m) => m.as_reactive_component().post_setup_pass(ctx),
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
      Self::Foreign(m) => m.as_reactive_component().build(builder),
    }
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    match self {
      Self::Attributes(m) => m.as_reactive_component().post_build(builder),
      Self::Foreign(m) => m.as_reactive_component().post_build(builder),
    }
  }
}
