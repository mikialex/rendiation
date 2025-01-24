use std::borrow::Cow;

use crate::*;

// fn create_blas(device: &GPUDevice) {
//   let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//     label: Some("Vertex Buffer"),
//     contents: bytemuck::cast_slice(&vertex_data),
//     usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::BLAS_INPUT,
//   });
//
//   let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//     label: Some("Index Buffer"),
//     contents: bytemuck::cast_slice(&index_data),
//     usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::BLAS_INPUT,
//   });
//
//   let blas_geo_size_desc = wgpu::BlasTriangleGeometrySizeDescriptor {
//     vertex_format: wgpu::VertexFormat::Float32x3,
//     vertex_count: vertex_data.len() as u32,
//     index_format: Some(wgpu::IndexFormat::Uint16),
//     index_count: Some(index_data.len() as u32),
//     flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
//   };
//
//   let blas = device.create_blas(
//     &wgpu::CreateBlasDescriptor {
//       label: None,
//       flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
//       update_mode: wgpu::AccelerationStructureUpdateMode::Build,
//     },
//     wgpu::BlasGeometrySizeDescriptors::Triangles {
//       descriptors: vec![blas_geo_size_desc.clone()],
//     },
//   );
//
//   // todo builder
//
//   // todo return a blas instance
// }

fn create_tlas(device: &GPUDevice) {
  // instance array
  // create tlas
  // return builder
}

pub struct DeviceBlas {
  dirty: bool,
  blas: Blas,
}
impl DeviceBlas {
  pub fn create(
    device: &GPUDevice,
    sources: &[BottomLevelAccelerationStructureBuildSource],
  ) -> Self {
    let mut vertex_buffer: Vec<f32> = vec![];
    let mut index_buffer: Vec<u32> = vec![];
    let mut size_descriptors: Vec<BlasTriangleGeometrySizeDescriptor> = vec![];

    for source in sources {
      match &source.geometry {
        BottomLevelAccelerationStructureBuildBuffer::Triangles { positions, indices } => {
          use bytemuck::cast_slice;
          vertex_buffer.extend_from_slice(cast_slice(positions));
          let index_count = match indices.as_ref() {
            None => {
              index_buffer.extend(0..positions.len() as u32);
              positions.len()
            }
            Some(indices) => {
              index_buffer.extend_from_slice(indices);
              indices.len()
            }
          };

          // this is all non-buffer data
          size_descriptors.push(BlasTriangleGeometrySizeDescriptor {
            vertex_format: VertexFormat::Float32x3,
            vertex_count: positions.len() as u32,
            index_format: Some(IndexFormat::Uint32),
            index_count: Some(index_count as u32),
            // GeometryFlags === AccelerationStructureGeometryFlags
            flags: AccelerationStructureGeometryFlags::from_bits(source.flags as u8).unwrap(),
          })
        }
        BottomLevelAccelerationStructureBuildBuffer::AABBs { .. } => {}
      }
    }

    let blas = device.create_blas(
      &CreateBlasDescriptor {
        label: None,
        flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
        update_mode: AccelerationStructureUpdateMode::Build,
      },
      BlasGeometrySizeDescriptors::Triangles {
        descriptors: size_descriptors,
      },
    );

    // todo create buffer uploader, blas builder

    todo!()
  }
  pub fn build(&self) {}
}

pub struct DeviceTlas {
  dirty: bool,
  tlas: GPUTlas,
}
impl DeviceTlas {
  pub fn create(
    device: &GPUDevice,
    sources: &[TopLevelAccelerationStructureSourceInstance],
  ) -> Self {
    // GPUTlas::create()
    todo!()
  }
  pub fn bind(&self) {
    // check build
    // bind
  }
}
