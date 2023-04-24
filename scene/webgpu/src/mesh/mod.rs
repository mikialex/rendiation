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

pub trait WebGPUSceneMesh: Any + Send + Sync {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMeshCache,
    sub_res: &mut AnyMap,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny;

  fn topology(&self) -> webgpu::PrimitiveTopology;
  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand;

  // the reason we use CPS style is for supporting refcell
  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh));
}

impl WebGPUSceneMesh for SceneMeshType {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMeshCache,
    sub_res: &mut AnyMap,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny {
    match self {
      SceneMeshType::AttributesMesh(m) => m.check_update_gpu(res, sub_res, gpu),
      SceneMeshType::Foreign(mesh) => {
        if let Some(mesh) = mesh.downcast_ref::<Box<dyn WebGPUSceneMesh>>() {
          mesh.check_update_gpu(res, sub_res, gpu)
        } else {
          &()
        }
      }
      _ => &(),
    }
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

impl<M: WebGPUMesh> WebGPUSceneMesh for Identity<M> {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMeshCache,
    sub_res: &mut AnyMap,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny {
    res.update_mesh(self, gpu, sub_res)
  }
  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    self.deref().draw_impl(group)
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.deref().topology()
  }
  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    self.deref().try_pick(f)
  }
}

impl GPUMeshCache {
  pub fn update_mesh<M: WebGPUMesh>(
    &mut self,
    m: &Identity<M>,
    gpu: &GPU,
    storage: &mut AnyMap,
  ) -> &dyn RenderComponentAny {
    let type_id = TypeId::of::<M>();

    let mapper = self
      .inner
      .entry(type_id)
      .or_insert_with(|| Box::<MeshIdentityMapper<M>>::default())
      .downcast_mut::<MeshIdentityMapper<M>>()
      .unwrap();
    mapper.get_update_or_insert_with_logic(m, |x| match x {
      ResourceLogic::Create(m) => ResourceLogicResult::Create(m.create(gpu, storage)),
      ResourceLogic::Update(gpu_m, m) => {
        m.update(gpu_m, gpu, storage);
        ResourceLogicResult::Update(gpu_m)
      }
    })
  }
}

type MeshIdentityMapper<T> = IdentityMapper<<T as WebGPUMesh>::GPU, T>;
pub trait WebGPUMesh: Any + Send + Sync + Incremental {
  type GPU: RenderComponent;
  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &GPU, storage: &mut AnyMap);
  fn create(&self, gpu: &GPU, storage: &mut AnyMap) -> Self::GPU;
  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand;

  fn topology(&self) -> webgpu::PrimitiveTopology;

  fn try_pick(&self, _f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {}
}

impl<T: WebGPUMesh + Any> WebGPUSceneMesh for SceneItemRef<T> {
  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.read().topology()
  }

  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    let inner = self.read();
    inner.try_pick(f);
  }

  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMeshCache,
    sub_res: &mut AnyMap,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny {
    let inner = self.read();
    inner.check_update_gpu(res, sub_res, gpu)
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    self.read().draw_impl(group)
  }
}

impl<T> WebGPUMesh for SceneItemRef<T>
where
  T: WebGPUMesh,
{
  type GPU = T::GPU;

  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &GPU, res: &mut AnyMap) {
    self.read().update(gpu_mesh, gpu, res);
  }

  fn create(&self, gpu: &GPU, res: &mut AnyMap) -> Self::GPU {
    self.read().create(gpu, res)
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    self.read().draw_impl(group)
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.read().topology()
  }

  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    self.read().try_pick(f)
  }
}

#[pin_project::pin_project(project = MaterialGPUInstanceProj)]
pub enum MeshGPUInstance {
  Attributes(ReactiveMaterialGPUOf<PhysicalMetallicRoughnessMaterial>),
  Foreign(Box<dyn ReactiveRenderComponentSource>),
}

impl ReactiveRenderComponent for MeshGPUInstance {
  fn create_render_component_delta_stream(
    &self,
  ) -> Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>> {
    match self {
      Self::Attributes(m) => Box::pin(m.as_ref().create_render_component_delta_stream())
        as Pin<Box<dyn Stream<Item = RenderComponentDeltaFlag>>>,
      Self::Foreign(m) => m
        .as_material_gpu_instance()
        .create_render_component_delta_stream(),
    }
  }
}

impl ShaderHashProvider for MeshGPUInstance {
  #[rustfmt::skip]
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    match self {
      Self::Attributes(m) => m.as_material_gpu_instance().hash_pipeline(hasher),
      Self::Foreign(m) => m.as_material_gpu_instance().hash_pipeline(hasher),
    }
  }
}

impl ShaderPassBuilder for MeshGPUInstance {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      Self::Attributes(m) => m.as_material_gpu_instance().setup_pass(ctx),
      Self::Foreign(m) => m.as_material_gpu_instance().setup_pass(ctx),
    }
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    match self {
      Self::Attributes(m) => m.as_material_gpu_instance().post_setup_pass(ctx),
      Self::Foreign(m) => m.as_material_gpu_instance().post_setup_pass(ctx),
    }
  }
}

impl ShaderGraphProvider for MeshGPUInstance {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    match self {
      Self::Attributes(m) => m.as_material_gpu_instance().build(builder),
      Self::Foreign(m) => m.as_material_gpu_instance().build(builder),
    }
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    match self {
      Self::Attributes(m) => m.as_material_gpu_instance().post_build(builder),
      Self::Foreign(m) => m.as_material_gpu_instance().post_build(builder),
    }
  }
}
