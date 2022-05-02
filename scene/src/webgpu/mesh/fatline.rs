use std::rc::Rc;

use crate::*;
use anymap::AnyMap;
use rendiation_algebra::*;
use rendiation_webgpu::*;

use rendiation_webgpu::util::DeviceExt;

use rendiation_renderable_mesh::{
  group::{GroupedMesh, MeshDrawGroup},
  mesh::{AbstractMesh, IndexedMesh, IntersectAbleGroupedMesh, NoneIndexedMesh, TriangleList},
  vertex::Vertex,
  MeshGPU,
};

pub struct FatlineMesh {
  inner: GroupedMesh<NoneIndexedMesh<FatLineVertex>>,
}

impl FatlineMesh {
  pub fn new(inner: GroupedMesh<NoneIndexedMesh<FatLineVertex>>) -> Self {
    Self { inner }
  }
}

impl WebGPUMesh for FatlineMesh {
  type GPU = FatlineMeshGPU;

  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &GPU, storage: &mut AnyMap) {
    *gpu_mesh = self.create(gpu, storage)
  }

  fn create(&self, gpu: &GPU, storage: &mut AnyMap) -> Self::GPU {
    let vertex = bytemuck::cast_slice(self.inner.mesh.data.as_slice());
    let vertex = gpu
      .device
      .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: vertex,
        usage: wgpu::BufferUsages::VERTEX,
      });
    let vertex = Rc::new(vertex);

    let instance = storage
      .entry()
      .or_insert_with(|| create_fatline_quad_gpu(&gpu.device))
      .data
      .clone();

    FatlineMeshGPU { vertex, instance }
  }

  fn draw_impl<'a>(&self, pass: &mut GPURenderPass<'a>, group: MeshDrawGroup) {
    FATLINE_INSTANCE.with(|instance| {
      pass.draw_indexed(
        0..instance.draw_count() as u32,
        0,
        self.inner.get_group(group).into(),
      )
    })
  }

  fn topology(&self) -> wgpu::PrimitiveTopology {
    wgpu::PrimitiveTopology::TriangleList
  }

  fn try_pick(&self, _f: &mut dyn FnMut(&dyn IntersectAbleGroupedMesh)) {}
}

impl ShaderGraphProvider for FatlineMeshGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, _| {
      builder.register_vertex::<Vertex>(VertexStepMode::Vertex);
      builder.register_vertex::<FatLineVertex>(VertexStepMode::Instance);
      builder.primitive_state.topology = wgpu::PrimitiveTopology::TriangleList;
      Ok(())
    })
  }
}

impl ShaderHashProvider for FatlineMeshGPU {}

impl ShaderPassBuilder for FatlineMeshGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.instance.setup_pass(&mut ctx.pass);
    ctx.pass.set_vertex_buffer_owned(1, &self.vertex);
  }
}

pub struct FatlineMeshGPU {
  vertex: Rc<wgpu::Buffer>,
  /// All fatline gpu instance shall share one instance buffer
  instance: Rc<MeshGPU>,
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod, ShaderVertex)]
pub struct FatLineVertex {
  #[semantic(FatLineStart)]
  pub start: Vec3<f32>,
  #[semantic(FatLineEnd)]
  pub end: Vec3<f32>,
  #[semantic(GeometryColorWithAlpha)]
  pub color: Vec4<f32>,
}

pub struct FatLineStart;
impl SemanticVertexShaderValue for FatLineStart {
  type ValueType = Vec3<f32>;
}
pub struct FatLineEnd;
impl SemanticVertexShaderValue for FatLineEnd {
  type ValueType = Vec3<f32>;
}

pub struct FatlineQuadInstance {
  data: Rc<MeshGPU>,
}

fn create_fatline_quad() -> IndexedMesh<u16, Vertex, TriangleList> {
  #[rustfmt::skip]
  let positions: Vec<isize> = vec![- 1, 2, 0, 1, 2, 0, - 1, 1, 0, 1, 1, 0, - 1, 0, 0, 1, 0, 0, - 1, - 1, 0, 1, - 1, 0];
  let positions: &[Vec3<isize>] = bytemuck::cast_slice(positions.as_slice());
  let uvs: Vec<isize> = vec![-1, 2, 1, 2, -1, 1, 1, 1, -1, -1, 1, -1, -1, -2, 1, -2];
  let uvs: &[Vec2<isize>] = bytemuck::cast_slice(uvs.as_slice());

  let data: Vec<_> = positions
    .iter()
    .zip(uvs)
    .map(|(position, uv)| Vertex {
      position: position.map(|v| v as f32),
      normal: Vec3::new(0., 0., 1.),
      uv: uv.map(|v| v as f32),
    })
    .collect();

  let index = vec![0, 2, 1, 2, 3, 1, 2, 4, 3, 4, 5, 3, 4, 6, 5, 6, 7, 5];
  IndexedMesh::new(data, index)
}

thread_local! {
  static FATLINE_INSTANCE: IndexedMesh<u16, Vertex, TriangleList> = create_fatline_quad()
}

fn create_fatline_quad_gpu(device: &wgpu::Device) -> FatlineQuadInstance {
  FatlineQuadInstance {
    data: Rc::new(FATLINE_INSTANCE.with(|f| f.create_gpu(device))),
  }
}
