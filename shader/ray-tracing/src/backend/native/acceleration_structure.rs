use std::sync::RwLock;

use crate::*;

#[derive(Clone)]
struct BlasBuilder {
  blas: Blas,
  buffers: Vec<(
    GPUBufferResource,
    Option<GPUBufferResource>,
    BlasTriangleGeometrySizeDescriptor,
  )>,
}
impl BlasBuilder {
  fn make_build_entry(&self) -> BlasBuildEntry<'_> {
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
              vertex_stride: 12, // xyz
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
struct DeviceBlas {
  blas: Blas,
}
impl DeviceBlas {
  fn create(
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
struct TlasBuilder {
  tlas: GPUTlas,
}
impl TlasBuilder {
  fn make_build_entry(&self) -> &TlasPackage {
    self.tlas.gpu_resource()
  }
}

#[derive(Clone)]
struct DeviceTlas {
  tlas: GPUTlas,
}
impl DeviceTlas {
  fn create(
    device: &GPUDevice,
    sources: &[TopLevelAccelerationStructureSourceInstance],
    blas_list: &[Option<DeviceBlas>],
  ) -> (Self, TlasBuilder) {
    let source = GPUTlasSource {
      instances: sources
        .iter()
        .map(|source| {
          let blas = &blas_list[source.acceleration_structure_handle.0 as usize];
          assert!(blas.is_some());
          blas.as_ref().map(|blas| {
            let blas = &blas.blas;
            let right = source.transform.right();
            let up = source.transform.up();
            let forward = source.transform.forward();
            let position = source.transform.position();
            let transform = [
              right.x, up.x, forward.x, position.x, right.y, up.y, forward.y, position.y, right.z,
              up.z, forward.z, position.z,
            ];
            TlasInstance::new(
              blas,
              transform,
              source.instance_custom_index,
              source.mask as u8,
            )
          })
        })
        .collect(),
      flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
      update_mode: AccelerationStructureUpdateMode::Build,
    };
    let gpu_tlas = GPUTlas::create(source, device);
    (
      DeviceTlas {
        tlas: gpu_tlas.clone(),
      },
      TlasBuilder { tlas: gpu_tlas },
    )
  }
  fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> HandleNode<ShaderAccelerationStructure> {
    compute_cx.bind_by(&self.tlas.create_default_view())
  }
  fn bind(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.tlas.create_default_view());
  }
}

#[derive(Clone)]
pub struct NativeInlineSystem {
  inner: Arc<RwLock<NativeInlineSystemInner>>,
}
impl NativeInlineSystem {
  pub fn new(gpu: &GPUDevice) -> Self {
    Self {
      inner: Arc::new(RwLock::new(NativeInlineSystemInner {
        device: gpu.clone(),
        tlas_binding: vec![],
        blas: vec![],
        tlas: vec![],
        blas_builders: vec![],
        tlas_builders: vec![],
      })),
    }
  }
  pub fn maintain(&self, encoder: &mut GPUCommandEncoder) {
    self.inner.write().unwrap().maintain(encoder);
  }
  pub fn create_instance(&self) -> NativeInlineInstance {
    let this = self.inner.read().unwrap();
    let tlas_bindings = this
      .tlas_binding
      .iter()
      .map(|i| {
        let tlas = this.tlas[i.0 as usize].as_ref().unwrap();
        tlas.clone()
      })
      .collect();

    NativeInlineInstance { tlas_bindings }
  }
}
#[derive(Clone)]
pub struct NativeInlineSystemInner {
  device: GPUDevice,
  tlas_binding: Vec<TlasHandle>,

  blas: Vec<Option<DeviceBlas>>,
  // blas_freelist: Vec<BlasHandle>,
  tlas: Vec<Option<DeviceTlas>>,
  // tlas_freelist: Vec<TlasHandle>,
  blas_builders: Vec<BlasBuilder>,
  tlas_builders: Vec<TlasBuilder>,
}

impl NativeInlineSystemInner {
  fn maintain(&mut self, encoder: &mut GPUCommandEncoder) {
    if self.blas_builders.is_empty() && self.tlas_builders.is_empty() {
      return;
    }

    let blas_entries = self
      .blas_builders
      .iter()
      .map(|builder| builder.make_build_entry())
      .collect::<Vec<_>>();
    let tlas_entries = self
      .tlas_builders
      .iter()
      .map(|builder| builder.make_build_entry())
      .collect::<Vec<_>>();

    encoder.build_acceleration_structures(blas_entries.iter(), tlas_entries);

    self.blas_builders.clear();
    self.tlas_builders.clear();
  }

  fn bind_tlas_max_len() -> u32 {
    4
  }
  fn bind_tlas(&mut self, tlas: &[TlasHandle]) {
    assert!(tlas.len() <= NativeInlineSystemInner::bind_tlas_max_len() as usize);
    self.tlas_binding = tlas.to_vec();
  }

  fn create_tlas(&mut self, source: &[TopLevelAccelerationStructureSourceInstance]) -> TlasHandle {
    let (tlas, builder) = DeviceTlas::create(&self.device, source, &self.blas);
    let handle = TlasHandle(self.tlas.len() as u32);
    self.tlas.push(Some(tlas));
    self.tlas_builders.push(builder);
    handle
  }

  fn delete_tlas(&mut self, id: TlasHandle) {
    self.tlas[id.0 as usize] = None;
  }

  fn create_blas(&mut self, source: &[BottomLevelAccelerationStructureBuildSource]) -> BlasHandle {
    let (blas, builder) = DeviceBlas::create(&self.device, source);
    let handle = BlasHandle(self.blas.len() as u32);
    self.blas.push(Some(blas));
    self.blas_builders.push(builder);
    handle
  }

  fn delete_blas(&mut self, id: BlasHandle) {
    self.blas[id.0 as usize] = None;
  }
}

impl GPUAccelerationStructureSystemProvider for NativeInlineSystem {
  fn create_comp_instance(&self) -> Box<dyn GPUAccelerationStructureSystemCompImplInstance> {
    Box::new(self.create_instance())
  }

  fn bind_tlas_max_len(&self) -> u32 {
    NativeInlineSystemInner::bind_tlas_max_len()
  }

  fn bind_tlas(&self, tlas: &[TlasHandle]) {
    self.inner.write().unwrap().bind_tlas(tlas);
  }

  fn create_top_level_acceleration_structure(
    &self,
    source: &[TopLevelAccelerationStructureSourceInstance],
  ) -> TlasHandle {
    self.inner.write().unwrap().create_tlas(source)
  }

  fn delete_top_level_acceleration_structure(&self, id: TlasHandle) {
    self.inner.write().unwrap().delete_tlas(id)
  }

  fn create_bottom_level_acceleration_structure(
    &self,
    source: &[BottomLevelAccelerationStructureBuildSource],
  ) -> BlasHandle {
    self.inner.write().unwrap().create_blas(source)
  }

  fn delete_bottom_level_acceleration_structure(&self, id: BlasHandle) {
    self.inner.write().unwrap().delete_blas(id)
  }
}

pub struct NativeInlineInstance {
  tlas_bindings: Vec<DeviceTlas>,
}
pub struct NativeInlineInvocation {
  tlas_bindings: Vec<HandleNode<ShaderAccelerationStructure>>,
}
impl GPUAccelerationStructureSystemCompImplInstance for NativeInlineInstance {
  fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn GPUAccelerationStructureSystemCompImplInvocationTraversable> {
    let handle_list = self
      .tlas_bindings
      .iter()
      .map(|tlas| tlas.build_shader(compute_cx))
      .collect();
    Box::new(NativeInlineInvocation {
      tlas_bindings: handle_list,
    })
  }

  fn bind_pass(&self, builder: &mut BindingBuilder) {
    for tlas in &self.tlas_bindings {
      tlas.bind(builder);
    }
  }

  fn create_tlas_instance(&self) -> Box<dyn GPUAccelerationStructureSystemTlasCompImplInstance> {
    todo!()
  }
}
impl GPUAccelerationStructureSystemCompImplInvocationTraversable for NativeInlineInvocation {
  fn traverse(
    &self,
    trace_payload: ENode<ShaderRayTraceCallStoragePayload>,
    _user_defined_payloads: StorageNode<[u32]>,
    _intersect: &dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter),
    _any_hit: &dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
  ) -> ShaderOption<RayClosestHitCtx> {
    // todo anyhit ctx
    // let user_defined_payload = U32BufferLoadStoreSource {
    //   array: user_defined_payloads,
    //   offset: trace_payload.payload_ref,
    // };

    // let tlas = trace_payload.tlas_idx todo a simple switch case tlas_idx -> tlas_bindings
    let query = Node::<ShaderRayQuery>::initialize(
      self.tlas_bindings[0], // todo select the first for now
      trace_payload.ray_flags,
      trace_payload.cull_mask,
      trace_payload.range.x(),
      trace_payload.range.y(),
      trace_payload.ray_origin,
      trace_payload.ray_direction,
    );

    loop_by(|ctx| {
      let any_candidate = query.proceed();
      if_by(any_candidate, || {
        // todo call anyhit
      })
      .else_by(|| ctx.do_break());
    });

    let intersection = query.get_commited_intersection();

    // todo support hit generated
    let hit_triangle = intersection
      .kind()
      .equals(val(RayIntersectionKind::Triangle as u32));

    ShaderOption {
      is_some: hit_triangle,
      payload: RayClosestHitCtx {
        launch_info: RayLaunchInfo {
          launch_id: trace_payload.launch_id,
          launch_size: trace_payload.launch_size,
        },
        world_ray: WorldRayInfo {
          world_ray: ShaderRay {
            origin: trace_payload.ray_origin,
            direction: trace_payload.ray_direction,
          },
          ray_range: ShaderRayRange {
            min: trace_payload.range.x(),
            max: trace_payload.range.y(),
          },
          ray_flags: trace_payload.ray_flags,
        },
        hit_ctx: HitCtxInfo {
          primitive_id: intersection.primitive_index(),
          instance_id: intersection.instance_id(),
          instance_sbt_offset: intersection.sbt_record_offset(),
          instance_custom_id: intersection.instance_custom_index(),
          geometry_id: intersection.geometry_index(),
          // object_to_world: (), todo load tlas meta with instance id. see traverse_gpu
          // world_to_object: (), todo or we reuse ray intersection 4x3 matrices
          object_space_ray: ShaderRay {
            // todo transform ray
            origin: val(vec3(0., 0., 0.)),
            direction: val(vec3(0., 0., 0.)),
          },
        },
        hit: HitInfo {
          hit_kind: intersection.front_face().select(
            val(HIT_KIND_FRONT_FACING_TRIANGLE),
            val(HIT_KIND_BACK_FACING_TRIANGLE),
          ),
          hit_distance: intersection.t(),
          hit_attribute: BuiltInTriangleHitAttributeShaderAPIInstance {
            bary_coord: intersection.barycentrics(),
          }
          .construct(),
        },
      },
    }
  }
}
