use crate::*;

#[derive(Clone)]
pub struct BlasBuilder {
  blas: Blas,
  buffers: Vec<(
    GPUBufferResource,
    Option<GPUBufferResource>,
    BlasTriangleGeometrySizeDescriptor,
  )>,
}
impl BlasBuilder {
  pub fn make_build_entry(&self) -> BlasBuildEntry<'_> {
    BlasBuildEntry {
      blas: &self.blas,
      geometry: BlasGeometries::TriangleGeometries(
        self
          .buffers
          .iter()
          .map(
            |(vertex_buffer, index_buffer, size_desc)| BlasTriangleGeometry {
              size: size_desc,
              vertex_buffer: vertex_buffer.gpu(),
              first_vertex: 0,
              vertex_stride: 12,
              index_buffer: index_buffer.as_ref().map(|b| b.gpu()),
              first_index: index_buffer.as_ref().map(|_| 0),
              transform_buffer: None,
              transform_buffer_offset: None,
            },
          )
          .collect(),
      ),
    }
  }
}

#[derive(Clone)]
pub struct DeviceBlas {
  blas: Blas,
}
impl DeviceBlas {
  pub fn create(
    device: &GPUDevice,
    sources: &[BottomLevelAccelerationStructureBuildSource],
  ) -> (Self, BlasBuilder) {
    let mut buffers = vec![];
    let mut size_descriptors: Vec<BlasTriangleGeometrySizeDescriptor> = vec![];

    for source in sources {
      match &source.geometry {
        BottomLevelAccelerationStructureBuildBuffer::Triangles { positions, indices } => {
          use bytemuck::cast_slice;

          let vertex_buffer =
            create_gpu_buffer(cast_slice(positions), BufferUsages::BLAS_INPUT, device);
          let mut index_buffer = None;

          let index_len = indices.as_ref().map(|indices| {
            index_buffer = Some(create_gpu_buffer(
              cast_slice(indices),
              BufferUsages::BLAS_INPUT,
              device,
            ));
            indices.len()
          });

          // this is all non-buffer data
          let size_desc = BlasTriangleGeometrySizeDescriptor {
            vertex_format: VertexFormat::Float32x3,
            vertex_count: positions.len() as u32,
            index_format: index_len.map(|_| IndexFormat::Uint32),
            index_count: index_len.map(|i| i as u32),
            // GeometryFlags === AccelerationStructureGeometryFlags
            flags: AccelerationStructureGeometryFlags::from_bits(source.flags as u8).unwrap(),
          };
          size_descriptors.push(size_desc.clone());

          buffers.push((vertex_buffer, index_buffer, size_desc));
        }
        BottomLevelAccelerationStructureBuildBuffer::AABBs { .. } => {
          unimplemented!()
        }
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

    (
      DeviceBlas { blas: blas.clone() },
      BlasBuilder { blas, buffers },
    )
  }
}

#[derive(Clone)]
pub struct DeviceTlas {
  tlas: GPUTlas,
}
impl DeviceTlas {
  pub fn create(
    device: &GPUDevice,
    sources: &[TopLevelAccelerationStructureSourceInstance],
  ) -> Self {
    // let source = GPUTlasSource {
    //   instances: vec![],
    //   flags: (),
    //   update_mode: AccelerationStructureUpdateMode::Build,
    // };
    // GPUTlas::create()
    todo!()
  }
  pub fn bind(&self) {
    // check build
    // bind
  }
}
#[derive(Clone)]
struct TlasBuilder {}

#[derive(Clone)]
pub struct NativeAccelerationStructureSystemProvider {
  inner: Arc<RwLock<NativeAccelerationStructureSystemProviderInner>>,
}
#[derive(Clone)]
pub struct NativeAccelerationStructureSystemProviderInner {
  blas: Vec<Option<DeviceBlas>>,
  blas_freelist: Vec<BlasHandle>,
  tlas: Vec<Option<DeviceTlas>>,
  tlas_freelist: Vec<TlasHandle>,

  blas_builders: Vec<BlasBuilder>,
  tlas_builders: Vec<TlasBuilder>,
}
impl GPUAccelerationStructureSystemProvider for NativeAccelerationStructureSystemProvider {
  fn create_comp_instance(&self) -> Box<dyn GPUAccelerationStructureSystemCompImplInstance> {
    todo!()
  }

  fn create_top_level_acceleration_structure(
    &self,
    source: &[TopLevelAccelerationStructureSourceInstance],
  ) -> TlasHandle {
    todo!()
  }

  fn delete_top_level_acceleration_structure(&self, id: TlasHandle) {
    todo!()
  }

  fn create_bottom_level_acceleration_structure(
    &self,
    source: &[BottomLevelAccelerationStructureBuildSource],
  ) -> BlasHandle {
    todo!()
  }

  fn delete_bottom_level_acceleration_structure(&self, id: BlasHandle) {
    todo!()
  }
}
