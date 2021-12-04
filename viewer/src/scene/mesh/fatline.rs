use std::rc::Rc;

use crate::*;
use anymap::AnyMap;
use rendiation_algebra::*;
use rendiation_webgpu::*;

use rendiation_webgpu::util::DeviceExt;

use rendiation_renderable_mesh::{
  group::{GroupedMesh, MeshDrawGroup, MeshGroup},
  mesh::{AbstractMesh, IndexedMesh, NoneIndexedMesh, TriangleList},
  vertex::Vertex,
  MeshGPU,
};

pub type FatlineData = NoneIndexedMesh<FatLineVertex>;

pub struct FatlineMeshCellImpl {
  data: GroupedMesh<FatlineData>,
  gpu: Option<FatlineMeshGPU>,
}

pub struct FatlineMeshGPU {
  range_full: MeshGroup,
  vertex: Rc<wgpu::Buffer>,
  /// All fatline gpu instance shall share one instance buffer
  instance: Rc<MeshGPU>,
}

impl FatlineMeshGPU {
  pub fn setup_pass_and_draw<'a>(&self, pass: &mut GPURenderPass<'a>, range: Option<MeshGroup>) {
    let range = range.unwrap_or(self.range_full);

    self.instance.setup_pass(pass);

    pass.set_vertex_buffer_owned(1, &self.vertex);

    pass.draw(self.instance.get_range_full().into(), range.into());
  }
}

impl From<FatlineData> for FatlineMeshCellImpl {
  fn from(data: FatlineData) -> Self {
    Self {
      data: GroupedMesh::full(data),
      gpu: None,
    }
  }
}

impl Mesh for FatlineMeshCellImpl {
  fn setup_pass_and_draw<'a>(&self, pass: &mut GPURenderPass<'a>, group: MeshDrawGroup) {
    self
      .gpu
      .as_ref()
      .unwrap()
      .setup_pass_and_draw(pass, self.data.get_group(group).into())
  }

  fn update(&mut self, gpu: &GPU, storage: &mut AnyMap) {
    let cpu = &self.data.mesh;

    self.gpu.get_or_insert_with(|| {
      let range_full = MeshGroup {
        start: 0,
        count: cpu.draw_count(),
      };

      let vertex = bytemuck::cast_slice(cpu.data.as_slice());
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
        .or_insert_with(|| create_fatline_quad(&gpu.device))
        .data
        .clone();

      FatlineMeshGPU {
        range_full,
        vertex,
        instance,
      }
    });
  }

  fn vertex_layout(&self) -> Vec<VertexBufferLayoutOwned> {
    vec![FatLineVertex::vertex_layout(), Vertex::vertex_layout()]
  }

  fn topology(&self) -> wgpu::PrimitiveTopology {
    wgpu::PrimitiveTopology::TriangleList
  }
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct FatLineVertex {
  pub start: Vec3<f32>,
  pub end: Vec3<f32>,
  pub color: Vec3<f32>,
}

impl VertexBufferSourceType for FatLineVertex {
  fn vertex_layout() -> VertexBufferLayoutOwned {
    VertexBufferLayoutOwned {
      array_stride: std::mem::size_of::<Self>() as u64,
      step_mode: VertexStepMode::Instance,
      attributes: vec![
        VertexAttribute {
          format: VertexFormat::Float32x3,
          offset: 0,
          shader_location: 0,
        },
        VertexAttribute {
          format: VertexFormat::Float32x3,
          offset: 4 * 3,
          shader_location: 1,
        },
        VertexAttribute {
          format: VertexFormat::Float32x3,
          offset: 4 * 3 + 4 * 3,
          shader_location: 2,
        },
      ],
    }
  }

  fn get_shader_header() -> &'static str {
    r#"
      [[location(4)]] fatline_start: vec3<f32>,
      [[location(5)]] fatline_end: vec3<f32>,
      [[location(6)]] fatline_color: vec3<f32>,
    "#
  }
}

pub struct FatlineQuadInstance {
  data: Rc<MeshGPU>,
}

fn create_fatline_quad(device: &wgpu::Device) -> FatlineQuadInstance {
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

  let mesh: IndexedMesh<u16, Vertex, TriangleList> = IndexedMesh::new(data, index);
  FatlineQuadInstance {
    data: Rc::new(mesh.create_gpu(device)),
  }
}
