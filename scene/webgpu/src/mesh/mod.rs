use anymap::AnyMap;
use rendiation_renderable_mesh::{
  group::MeshDrawGroup, mesh::IntersectAbleGroupedMesh, GPUMeshData, TypedMeshGPU,
};
use std::{
  any::{Any, TypeId},
  ops::Deref,
};
use webgpu::GPU;

use rendiation_renderable_mesh::{group::GroupedMesh, mesh::IndexedMesh};

pub mod fatline;
pub use fatline::*;
pub mod transform_instance;
pub use transform_instance::*;

use crate::*;

pub trait GPUMeshLayoutSupport {
  type VertexInput;
}

impl<I, V, T> GPUMeshLayoutSupport for GroupedMesh<IndexedMesh<I, V, T, Vec<V>>> {
  type VertexInput = Vec<V>;
}

pub trait WebGPUSceneMesh: 'static {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMeshCache,
    sub_res: &mut AnyMap,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny;

  fn topology(&self) -> webgpu::PrimitiveTopology;
  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand;

  // the reason we use CPS style is for supporting refcell
  fn try_pick(&self, _f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {}
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
      .or_insert_with(|| Box::new(MeshIdentityMapper::<M>::default()))
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
pub trait WebGPUMesh: Any {
  type GPU: RenderComponent;
  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &GPU, storage: &mut AnyMap);
  fn create(&self, gpu: &GPU, storage: &mut AnyMap) -> Self::GPU;
  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand;

  fn topology(&self) -> webgpu::PrimitiveTopology;

  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh));
}

pub struct MeshSource<T> {
  inner: T,
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
  ) -> rendiation_geometry::Nearest<rendiation_renderable_mesh::mesh::MeshBufferHitPoint> {
    self.deref().intersect_nearest(ray, conf, group)
  }
}

impl<T> WebGPUMesh for MeshSource<T>
where
  T: GPUMeshData<GPU = TypedMeshGPU<T>> + IntersectAbleGroupedMesh + Any,
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

impl<T: WebGPUMesh + IntersectAbleGroupedMesh + Any> WebGPUSceneMesh for MeshCell<T> {
  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.inner.read().unwrap().topology()
  }

  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    let inner = self.inner.read().unwrap();
    inner.try_pick(f);
  }

  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMeshCache,
    sub_res: &mut AnyMap,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny {
    let inner = self.inner.read().unwrap();
    inner.check_update_gpu(res, sub_res, gpu)
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    self.inner.read().unwrap().draw_impl(group)
  }
}

impl<T> WebGPUMesh for MeshCell<T>
where
  T: WebGPUMesh,
{
  type GPU = T::GPU;

  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &GPU, res: &mut AnyMap) {
    self.inner.read().unwrap().update(gpu_mesh, gpu, res);
  }

  fn create(&self, gpu: &GPU, res: &mut AnyMap) -> Self::GPU {
    self.inner.read().unwrap().create(gpu, res)
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    self.inner.read().unwrap().draw_impl(group)
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.inner.read().unwrap().topology()
  }

  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    self.inner.read().unwrap().try_pick(f)
  }
}
