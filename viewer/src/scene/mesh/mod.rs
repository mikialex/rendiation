use anymap::AnyMap;
use rendiation_renderable_mesh::{group::MeshDrawGroup, GPUMeshData, MeshGPU};
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
}

pub struct MeshCellInner<T> {
  data: T,
  gpu: Option<MeshGPU>,
}

impl<T> MeshCellInner<T> {
  pub fn new(data: T) -> Self {
    Self { data, gpu: None }
  }
}

pub struct MeshCell<T> {
  inner: Rc<RefCell<MeshCellInner<T>>>,
}

impl<T> MeshCell<T> {
  pub fn new(mesh: T) -> Self {
    let mesh = MeshCellInner::new(mesh);
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

impl<T: GPUMeshData> Mesh for MeshCellInner<T> {
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
}

impl<T: GPUMeshData> Mesh for MeshCell<T> {
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
}

// impl Scene {
//   pub fn add_mesh<M>(&mut self, mesh: M) -> TypedMeshHandle<M>
//   where
//     M: GPUMeshData + 'static,
//   {
//     let handle = self
//       .components
//       .meshes
//       .insert(Box::new(MeshCellInner::from(mesh)));
//     TypedMeshHandle {
//       handle,
//       ty: PhantomData,
//     }
//   }
// }

// /// the comprehensive data that provided by mesh and will affect graphic pipeline
// pub struct MeshLayout {
//   vertex: Vec<wgpu::VertexBufferLayout<'static>>,
//   index: wgpu::IndexFormat,
//   topology: wgpu::PrimitiveTopology,
// }
