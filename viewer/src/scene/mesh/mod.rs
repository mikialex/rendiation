use anymap::AnyMap;
use rendiation_renderable_mesh::{
  group::MeshDrawGroup, mesh::IntersectAbleGroupedMesh, GPUMeshData, MeshGPU,
};
use rendiation_webgpu::{GPURenderPass, VertexBufferLayoutOwned, GPU};
use std::{cell::RefCell, rc::Rc};

use rendiation_renderable_mesh::{group::GroupedMesh, mesh::IndexedMesh};
use rendiation_webgpu::VertexBufferSourceType;

pub mod fatline;
pub use fatline::*;

pub trait GPUMeshLayoutSupport {
  type VertexInput;
}

impl<I, V, T> GPUMeshLayoutSupport for GroupedMesh<IndexedMesh<I, V, T, Vec<V>>>
where
  V: VertexBufferSourceType,
{
  type VertexInput = Vec<V>;
}

pub trait Mesh {
  fn setup_pass_and_draw<'a>(&self, pass: &mut GPURenderPass<'a>, group: MeshDrawGroup);
  fn update(&mut self, gpu: &GPU, storage: &mut AnyMap);
  fn vertex_layout(&self) -> Vec<VertexBufferLayoutOwned>;
  fn topology(&self) -> wgpu::PrimitiveTopology;

  // the reason we use CPS style is for supporting refcell
  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {}
}

pub struct MeshCellImpl<T> {
  data: T,
  gpu: Option<MeshGPU>,
}

impl<T> MeshCellImpl<T> {
  pub fn new(data: T) -> Self {
    Self { data, gpu: None }
  }
}

pub struct MeshCell<T> {
  inner: Rc<RefCell<MeshCellImpl<T>>>,
}

impl<T> MeshCell<T> {
  pub fn new(mesh: T) -> Self {
    let mesh = MeshCellImpl::new(mesh);
    Self {
      inner: Rc::new(RefCell::new(mesh)),
    }
  }
}

impl<T> Clone for MeshCell<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T: GPUMeshData + IntersectAbleGroupedMesh> Mesh for MeshCellImpl<T> {
  fn setup_pass_and_draw<'a>(&self, pass: &mut GPURenderPass<'a>, group: MeshDrawGroup) {
    let gpu = self.gpu.as_ref().unwrap();
    gpu.setup_pass(pass);
    gpu.draw(pass, self.data.get_group(group).into())
  }

  fn update(&mut self, gpu: &GPU, _storage: &mut AnyMap) {
    self.data.update(&mut self.gpu, &gpu.device);
  }

  fn vertex_layout(&self) -> Vec<VertexBufferLayoutOwned> {
    self.data.vertex_layout()
  }

  fn topology(&self) -> wgpu::PrimitiveTopology {
    self.data.topology()
  }

  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    f(&self.data)
  }
}

impl<T: GPUMeshData + IntersectAbleGroupedMesh> Mesh for MeshCell<T> {
  fn setup_pass_and_draw<'a>(&self, pass: &mut GPURenderPass<'a>, group: MeshDrawGroup) {
    let inner = self.inner.borrow();
    inner.setup_pass_and_draw(pass, group);
  }

  fn update(&mut self, gpu: &GPU, storage: &mut AnyMap) {
    let mut inner = self.inner.borrow_mut();
    inner.update(gpu, storage)
  }

  fn vertex_layout(&self) -> Vec<VertexBufferLayoutOwned> {
    let inner = self.inner.borrow();
    inner.vertex_layout()
  }

  fn topology(&self) -> wgpu::PrimitiveTopology {
    self.inner.borrow().topology()
  }

  fn try_pick(&self, f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {
    let inner = self.inner.borrow();
    inner.try_pick(f);
  }
}
