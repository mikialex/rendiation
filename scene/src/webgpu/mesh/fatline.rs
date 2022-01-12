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

pub struct FatlineMesh {
  inner: GroupedMesh<NoneIndexedMesh<FatLineVertex>>,
}

impl MeshCPUSource for FatlineMesh {
  type GPU = FatlineMeshGPU;

  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &GPU, storage: &mut AnyMap) {}

  fn create(&self, gpu: &GPU, storage: &mut AnyMap) -> Self::GPU {
    let range_full = MeshGroup {
      start: 0,
      count: self.inner.mesh.draw_count(),
    };

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
      .or_insert_with(|| create_fatline_quad(&gpu.device))
      .data
      .clone();

    FatlineMeshGPU {
      range_full,
      vertex,
      instance,
    }
  }

  fn setup_pass_and_draw<'a>(
    &self,
    gpu: &Self::GPU,
    pass: &mut GPURenderPass<'a>,
    group: MeshDrawGroup,
  ) {
    gpu.setup_pass_and_draw(pass, self.inner.get_group(group).into())
  }

  fn vertex_layout(&self) -> Vec<VertexBufferLayoutOwned> {
    vec![Vertex::vertex_layout(), FatLineVertex::vertex_layout()]
  }

  fn topology(&self) -> wgpu::PrimitiveTopology {
    wgpu::PrimitiveTopology::TriangleList
  }
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

    pass.draw_indexed(self.instance.get_range_full().into(), 0, range.into());
  }
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct FatLineVertex {
  pub start: Vec3<f32>,
  pub end: Vec3<f32>,
  pub color: Vec4<f32>,
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
          shader_location: 4,
        },
        VertexAttribute {
          format: VertexFormat::Float32x3,
          offset: 4 * 3,
          shader_location: 5,
        },
        VertexAttribute {
          format: VertexFormat::Float32x4,
          offset: 4 * 3 + 4 * 3,
          shader_location: 6,
        },
      ],
    }
  }

  fn get_shader_header() -> &'static str {
    r#"
      [[location(4)]] fatline_start: vec3<f32>,
      [[location(5)]] fatline_end: vec3<f32>,
      [[location(6)]] fatline_color: vec4<f32>,
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
