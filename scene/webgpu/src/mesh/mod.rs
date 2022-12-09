use rendiation_renderable_mesh::{mesh::IntersectAbleGroupedMesh, GPUMeshData, TypedMeshGPU};

pub mod fatline;
pub use fatline::*;
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
    cb: &mut dyn FnMut(&dyn RenderComponentAny),
  );

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
    cb: &mut dyn FnMut(&dyn RenderComponentAny),
  ) {
    match self {
      SceneMeshType::AttributesMesh(m) => m.check_update_gpu(res, sub_res, gpu, cb),
      SceneMeshType::Foreign(mesh) => {
        if let Some(mesh) = mesh.downcast_ref::<Box<dyn WebGPUSceneMesh>>() {
          mesh.check_update_gpu(res, sub_res, gpu, cb)
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

impl WebGPUSceneMesh for SceneMesh {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMeshCache,
    sub_res: &mut AnyMap,
    gpu: &GPU,
    cb: &mut dyn FnMut(&dyn RenderComponentAny),
  ) {
    self.read().check_update_gpu(res, sub_res, gpu, cb)
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.read().topology()
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    self.read().draw_impl(group)
  }

  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    self.read().try_pick(f)
  }
}

impl<M: WebGPUMesh + Incremental> WebGPUSceneMesh for Identity<M> {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMeshCache,
    sub_res: &mut AnyMap,
    gpu: &GPU,
    cb: &mut dyn FnMut(&dyn RenderComponentAny),
  ) {
    res.update_mesh(self, gpu, sub_res, cb)
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
  pub fn update_mesh<M: WebGPUMesh + Incremental>(
    &mut self,
    m: &Identity<M>,
    gpu: &GPU,
    storage: &mut AnyMap,
    cb: &mut dyn FnMut(&dyn RenderComponentAny),
  ) {
    let type_id = TypeId::of::<M>();

    let mapper = self
      .inner
      .entry(type_id)
      .or_insert_with(|| {
        let mapper = IdentityMapper::new(
          |source, source_delta, gpu| {
            todo!() //
          },
          |source| {
            todo!() //
          },
        );
        Box::new(mapper)
      })
      .as_any_mut()
      .downcast_mut::<MeshIdentityMapper<M>>()
      .unwrap();

    // mapper.get_update_or_insert_with_logic(m, |x| match x {
    //   ResourceLogic::Create(m) => ResourceLogicResult::Create(m.create(gpu, storage)),
    //   ResourceLogic::Update(gpu_m, m) => {
    //     m.update(gpu_m, gpu, storage);
    //     ResourceLogicResult::Update(gpu_m)
    //   }
    // })
  }
}

type MeshIdentityMapper<T> = IdentityMapper<<T as WebGPUMesh>::GPU, T>;
pub trait WebGPUMesh: Any + Send + Sync {
  type GPU: RenderComponent;
  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &GPU, storage: &mut AnyMap);
  fn create(&self, gpu: &GPU, storage: &mut AnyMap) -> Self::GPU;
  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand;

  fn topology(&self) -> webgpu::PrimitiveTopology;

  fn try_pick(&self, _f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {}
}

#[derive(Clone)]
pub struct MeshSource<T> {
  inner: T,
}

impl<T: Clone + Sync + Send> SimpleIncremental for MeshSource<T> {
  type Delta = Self;

  fn s_apply(&mut self, delta: Self::Delta) {
    todo!()
  }

  fn s_expand(&self, cb: impl FnMut(Self::Delta)) {
    todo!()
  }
}

impl<T> MeshSource<T> {
  pub fn new(inner: T) -> Self {
    Self { inner }
  }
}

impl<T> std::ops::Deref for MeshSource<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<T> std::ops::DerefMut for MeshSource<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl<T: IntersectAbleGroupedMesh> IntersectAbleGroupedMesh for MeshSource<T> {
  fn intersect_list(
    &self,
    ray: rendiation_geometry::Ray3,
    conf: &rendiation_renderable_mesh::mesh::MeshBufferIntersectConfig,
    result: &mut rendiation_renderable_mesh::mesh::MeshBufferHitList,
    group: MeshDrawGroup,
  ) {
    self.deref().intersect_list(ray, conf, result, group)
  }

  fn intersect_nearest(
    &self,
    ray: rendiation_geometry::Ray3,
    conf: &rendiation_renderable_mesh::mesh::MeshBufferIntersectConfig,
    group: MeshDrawGroup,
  ) -> rendiation_geometry::OptionalNearest<rendiation_renderable_mesh::mesh::MeshBufferHitPoint>
  {
    self.deref().intersect_nearest(ray, conf, group)
  }
}

impl<T> WebGPUMesh for MeshSource<T>
where
  T: GPUMeshData<GPU = TypedMeshGPU<T>> + IntersectAbleGroupedMesh + Any + Send + Sync,
{
  type GPU = TypedMeshGPU<T>;

  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &GPU, _: &mut AnyMap) {
    self.deref().update(gpu_mesh, &gpu.device);
  }

  fn create(&self, gpu: &GPU, _: &mut AnyMap) -> Self::GPU {
    self.deref().create(&gpu.device)
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    self.deref().draw(group)
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.deref().topology()
  }

  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    f(self.deref())
  }
}

impl<T: WebGPUMesh + Any + Incremental> WebGPUSceneMesh for SceneItemRef<T> {
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
    cb: &mut dyn FnMut(&dyn RenderComponentAny),
  ) {
    let inner = self.read();
    inner.check_update_gpu(res, sub_res, gpu, cb)
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    self.read().draw_impl(group)
  }
}

impl<T> WebGPUMesh for SceneItemRef<T>
where
  T: WebGPUMesh + Incremental,
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
