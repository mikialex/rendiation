use crate::*;

/// like AttributeAccessor, but for CKey usage.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttributeAccessKey {
  pub view: AllocIdx<GeometryBufferImpl>,
  pub range: BufferViewRange,
  /// offset relative to the view
  pub byte_offset: usize,
  pub count: usize,
  /// corespondent to the data type
  /// for example: vec3<f32> => 3 * 4
  pub item_byte_size: usize,
}

pub fn global_normalized_att_sematic_set(
) -> impl ReactiveCollection<(AllocIdx<AttributesMesh>, AttributeSemantic), ()> {
}

pub fn global_normalized_att_acc_keys(
) -> impl ReactiveCollection<(AllocIdx<AttributesMesh>, AttributeSemantic), AttributeAccessKey> {
}

pub fn global_acc_keys_set() -> impl ReactiveCollection<AttributeAccessKey, ()> {
  global_normalized_att_sematic_set().many_to_one_reduce_key(global_normalized_att_acc_keys())
}

// used for positional related compute
pub fn position_attributes(
  scope: impl ReactiveCollection<AllocIdx<AttributesMesh>, ()>,
) -> impl ReactiveCollection<AttributeAccessKey, ()> {
  // global_normalized_att_sematic_set filter out position key
  // reduce by global_normalized_att_acc_keys
  // remapping
}

pub fn vertex_attribute_buffers_scope(
  scope: impl ReactiveCollection<AllocIdx<AttributesMesh>, ()>,
) -> impl ReactiveCollection<AttributeAccessKey, ()> {

  //
}

pub fn index_attribute_buffers_scope(
  scope: impl ReactiveCollection<AllocIdx<AttributesMesh>, ()>,
) -> impl ReactiveCollection<AttributeAccessKey, ()> {
  //
}

pub fn gpu_attribute_vertex_buffers(
  gpu: &ResourceGPUCtx,
  scope: impl ReactiveCollection<AttributeAccessKey, ()>,
) -> impl ReactiveCollection<AttributeAccessKey, GPUBufferResourceView> {
  // scope.collective_execute_map_by(move || {
  //   let gpu = gpu.clone();
  //   let creator = storage_of::<GeometryBufferImpl>().create_key_mapper(move |m, _| mapper(m,
  // &gpu));   move |k, _| creator(*k)
  // })
  // storage_of::<AttributeAccessor>()
  //
}

pub fn gpu_attribute_index_buffers(
  cx: &ResourceGPUCtx,
  scope: impl ReactiveCollection<AttributeAccessKey, ()>,
) -> impl ReactiveCollection<AttributeAccessKey, GPUBufferResourceView> {
  // storage_of::<AttributeAccessor>()
  //
}

pub fn attribute_mesh_shader_keys(
  scope: impl ReactiveCollection<AllocIdx<AttributesMesh>, ()>,
) -> impl ReactiveCollection<AttributeAccessKey, u64> {
}

pub struct AttributesMeshGPU<'a> {
  mesh: &'a AttributesMesh,
  vertex_buffer_ctx: &'a MeshVertexBufferManager,
  index_buffer_ctx: &'a MeshIndexBufferManager,
}

impl<'a> ShaderPassBuilder for AttributesMeshGPU<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    for (_, b) in &self.mesh.attributes {
      ctx.set_vertex_buffer_owned_next(self.vertex_buffer_ctx.get_gpu_vertex(b));
    }
    if let Some((index_format, buffer)) = &self.mesh.indices {
      ctx.pass.set_index_buffer_owned(
        self.index_buffer_ctx.get_gpu_index(buffer),
        map_index(*index_format),
      )
    }
  }
}

pub trait CustomAttributeKeyGPU {
  fn inject_shader(&self, builder: &mut ShaderVertexBuilder);
}
define_dyn_trait_downcaster_static!(CustomAttributeKeyGPU);

impl<'a> ShaderHashProvider for AttributesMeshGPU<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    for (s, _) in &self.mesh.attributes {
      s.hash(hasher)
    }
    self.mesh.mode.hash(hasher);
    if let Some((f, _)) = &self.mesh.indices {
      if rendiation_mesh_core::PrimitiveTopology::LineStrip == self.mesh.mode
        || rendiation_mesh_core::PrimitiveTopology::TriangleStrip == self.mesh.mode
      {
        f.hash(hasher)
      }
    }
  }
}
impl<'a> GraphicsShaderProvider for AttributesMeshGPU<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    let mode = VertexStepMode::Vertex;
    builder.vertex(|builder, _| {
      for (s, _) in &self.mesh.attributes {
        match s {
          AttributeSemantic::Positions => {
            builder.push_single_vertex_layout::<GeometryPosition>(mode)
          }
          AttributeSemantic::Normals => builder.push_single_vertex_layout::<GeometryNormal>(mode),
          AttributeSemantic::Tangents => builder.push_single_vertex_layout::<GeometryTangent>(mode),
          AttributeSemantic::Colors(_) => builder.push_single_vertex_layout::<GeometryColor>(mode),
          AttributeSemantic::TexCoords(channel) => match channel {
            // support 3 channel should be enough
            0 => builder.push_single_vertex_layout::<GeometryUVChannel<0>>(mode),
            1 => builder.push_single_vertex_layout::<GeometryUVChannel<1>>(mode),
            2 => builder.push_single_vertex_layout::<GeometryUVChannel<2>>(mode),
            _ => return Err(ShaderBuildError::SemanticNotSupported),
          },
          AttributeSemantic::Joints(channel) => match channel {
            // support 4 channel should be enough
            0 => builder.push_single_vertex_layout::<JointIndexChannel<0>>(mode),
            1 => builder.push_single_vertex_layout::<JointIndexChannel<1>>(mode),
            2 => builder.push_single_vertex_layout::<JointIndexChannel<2>>(mode),
            3 => builder.push_single_vertex_layout::<JointIndexChannel<3>>(mode),
            _ => return Err(ShaderBuildError::SemanticNotSupported),
          },
          AttributeSemantic::Weights(channel) => match channel {
            // support 4 channel should be enough
            0 => builder.push_single_vertex_layout::<WeightChannel<0>>(mode),
            1 => builder.push_single_vertex_layout::<WeightChannel<1>>(mode),
            2 => builder.push_single_vertex_layout::<WeightChannel<2>>(mode),
            3 => builder.push_single_vertex_layout::<WeightChannel<3>>(mode),
            _ => return Err(ShaderBuildError::SemanticNotSupported),
          },
          AttributeSemantic::Foreign(key) => {
            get_dyn_trait_downcaster_static!(CustomAttributeKeyGPU)
              .downcast_ref(key.implementation.as_ref().as_any())
              .ok_or(ShaderBuildError::SemanticNotSupported)?
              .inject_shader(builder)
          }
        }
      }
      builder.primitive_state.topology = map_topology(self.mesh.mode);
      Ok(())
    })
  }
}

struct GPUAttributesBuffer {
  inner: GPUBufferResource,
}

impl GlobalIdReactiveSimpleMapping<GPUAttributesBuffer> for GeometryBuffer {
  type ChangeStream = impl Stream<Item = ()> + Unpin;
  type Ctx<'a> = ResourceGPUCtx;

  fn build(&self, gpu: &Self::Ctx<'_>) -> (GPUAttributesBuffer, Self::ChangeStream) {
    let gpu_buffer = create_gpu_buffer(
      self.read().buffer.as_slice(),
      BufferUsages::INDEX | BufferUsages::VERTEX,
      &gpu.device,
    );

    let gpu_buffer = GPUAttributesBuffer { inner: gpu_buffer };

    let change = self.unbound_listen_by(any_change);
    (gpu_buffer, change)
  }
}

// fn get_update_buffer<'a>(
//   storage: &'a mut AnyMap,
//   source: &GeometryBuffer,
//   gpu: &ResourceGPUCtx,
// ) -> &'a GPUBufferResource {
//   let cache: &mut ReactiveMap<GeometryBuffer, GPUAttributesBuffer> =
//     storage.entry().or_insert_with(Default::default);
//   &cache.get_with_update(source, gpu).inner
// }

// impl MeshDrawcallEmitter for AttributesMeshGPUReactive {
//   fn draw_command(&self, _group: MeshDrawGroup) -> DrawCommand {
//     let inner: &MaybeBindlessMesh<AttributesMeshGPU> = self.inner.as_ref();
//     match inner {
//       MaybeBindlessMesh::Traditional(inner) => inner.draw.clone(),
//       MaybeBindlessMesh::Bindless(_) => DrawCommand::Skip,
//     }
//   }
// }
/// the current represent do not have meaningful mesh draw group concept
fn draw_command(mesh: &AttributesMesh) -> DrawCommand {
  if let Some((_, indices)) = &mesh.indices {
    DrawCommand::Indexed {
      base_vertex: 0,
      indices: 0..indices.count as u32,
      instances: 0..1,
    }
  } else {
    let attribute = &mesh.attributes.last().unwrap().1;
    DrawCommand::Array {
      vertices: 0..attribute.count as u32,
      instances: 0..1,
    }
  }
}

// fn to_vec4(vec3: &[Vec3<f32>]) -> Vec<Vec4<f32>> {
//   vec3.iter().map(|v| Vec4::new(v.x, v.y, v.z, 0.0)).collect()
// }

// #[allow(clippy::collapsible_match)]
// pub fn support_bindless(
//   mesh: &AttributeMeshReadView,
//   sys: &GPUBindlessMeshSystem,
//   device: &GPUDevice,
//   queue: &GPUQueue,
// ) -> Option<MeshSystemMeshInstance> {
//   if rendiation_mesh_core::PrimitiveTopology::TriangleList != mesh.mode {
//     return None;
//   }

//   if let Some((fmt, index)) = &mesh.indices {
//     if let AttributeIndexFormat::Uint32 = fmt {
//       if mesh.attributes.len() != 3 {
//         return None;
//       }
//       let position = mesh.get_position();
//       let position = to_vec4(position);
//       if let Some(normal) = mesh.get_attribute(&AttributeSemantic::Normals) {
//         let normal = to_vec4(normal.visit_slice::<Vec3<f32>>()?);
//         if let Some(uv) = mesh.get_attribute(&AttributeSemantic::TexCoords(0)) {
//           return Some(
//             sys
//               .create_mesh_instance(
//                 BindlessMeshSource {
//                   index: index.visit_slice()?,
//                   position: &position,
//                   normal: &normal,
//                   uv: uv.visit_slice()?,
//                 },
//                 device,
//                 queue,
//               )
//               .unwrap(),
//           );
//         }
//       }
//     }
//   }
//   None
// }

// #[pin_project::pin_project]
// pub struct AttributesMeshGPUReactive {
//   #[pin]
//   pub inner: AttributesMeshGPUReactiveInner,
// }

// impl Stream for AttributesMeshGPUReactive {
//   type Item = RenderComponentDeltaFlag;

//   fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//     let this = self.project();
//     this.inner.poll_next(cx)
//   }
// }

// pub type AttributesMeshGPUReactiveInner = impl
// AsRef<RenderComponentCell<MaybeBindlessMesh<AttributesMeshGPU>>>
//   + Stream<Item = RenderComponentDeltaFlag>;

// impl WebGPUMesh for AttributesMesh {
//   type ReactiveGPU = AttributesMeshGPUReactive;

//   fn create_reactive_gpu(
//     source: &IncrementalSignalPtr<Self>,
//     ctx: &ShareBindableResourceCtx,
//   ) -> Self::ReactiveGPU {
//     let ctx = ctx.clone();

//     let create = move |mesh: &IncrementalSignalPtr<AttributesMesh>| {
//       let m = mesh.read();
//       let gpu = &ctx.gpu;
//       let m = unsafe { std::mem::transmute(&m.read()) }; // todo why?
//       if let Some(sys) = &ctx.bindless_mesh
//         && let Some(mesh) = support_bindless(m, sys, &gpu.device, &gpu.queue)
//       {
//         MaybeBindlessMesh::Bindless(mesh)
//       } else {
//         let mut custom_storage = ctx.custom_storage.write().unwrap();
//         let mesh = mesh.read();
//         let attributes = mesh
//           .attributes
//           .iter()
//           .map(|(s, vertices)| {
//             let buffer = get_update_buffer(&mut custom_storage, &vertices.view.buffer, &ctx.gpu);
//             let buffer_view = buffer.create_view(map_view(vertices.compute_gpu_buffer_range()));
//             (s.clone(), buffer_view)
//           })
//           .collect();

//         let indices = mesh.indices.as_ref().map(|(format, i)| {
//           let buffer = get_update_buffer(&mut custom_storage, &i.view.buffer, &ctx.gpu);
//           let buffer_view = buffer.create_view(map_view(i.compute_gpu_buffer_range()));
//           (buffer_view, map_index(*format))
//         });

//         MaybeBindlessMesh::Traditional(AttributesMeshGPU {
//           attributes,
//           indices,
//           topology: map_topology(mesh.mode),
//           draw: draw_command(&mesh),
//         })
//       }
//     };

//     let state = RenderComponentCell::new(create(source));

//     let inner = source
//       .single_listen_by::<()>(any_change_no_init)
//       .filter_map_sync(source.defer_weak())
//       .fold_signal(state, move |mesh, state| {
//         state.inner = create(&mesh);
//         RenderComponentDeltaFlag::all().into()
//       });

//     AttributesMeshGPUReactive { inner }
//   }
// }

// fn map_view(view: BufferViewRange) -> GPUBufferViewRange {
//   GPUBufferViewRange {
//     offset: view.offset,
//     size: view.size,
//   }
// }
