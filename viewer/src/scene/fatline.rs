use rendiation_algebra::*;
use rendiation_renderable_mesh::{
  mesh::{IndexedMesh, TriangleList},
  vertex::Vertex,
};

fn create_fatline_quad(device: &wgpu::Device) -> IndexedMesh<u16, Vertex, TriangleList> {
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
