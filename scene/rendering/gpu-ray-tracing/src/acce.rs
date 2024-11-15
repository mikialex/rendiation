use crate::*;

fn get_sub_buffer(buffer: &[u8], range: Option<BufferViewRange>) -> &[u8] {
  if let Some(range) = range {
    buffer.get(range.into_range(buffer.len())).unwrap()
  } else {
    buffer
  }
}

#[derive(Clone)]
struct BlasInstance {
  handle: BottomLevelAccelerationStructureHandle,
  sys: Box<dyn GPUAccelerationStructureSystemProvider>,
}

impl std::fmt::Debug for BlasInstance {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("BlasInstance")
      .field("handle", &self.handle)
      .finish()
  }
}

impl PartialEq for BlasInstance {
  fn eq(&self, other: &Self) -> bool {
    self.handle == other.handle
  }
}

impl Drop for BlasInstance {
  fn drop(&mut self) {
    self
      .sys
      .delete_bottom_level_acceleration_structure(self.handle);
  }
}

pub fn mesh_to_blas(
  acc_sys: Box<dyn GPUAccelerationStructureSystemProvider>,
) -> impl ReactiveQuery<Key = EntityHandle<AttributesMeshEntity>, Value = BlasInstance> {
  let PositionRelatedAttributeMeshQuery {
    indexed,
    none_indexed,
  } = attribute_mesh_position_query();

  indexed.collective_execute_map_by(move || {
    let acc_sys = acc_sys.clone();
    let buffer_accessor = global_entity_component_of::<BufferEntityData>().read();
    move |_, (position, index)| {
      let position_buffer = buffer_accessor.get(position.0.unwrap()).unwrap();
      let position_buffer = get_sub_buffer(position_buffer.as_slice(), position.1);
      let position_buffer = bytemuck::cast_slice(position_buffer);

      let index_buffer = buffer_accessor.get(index.0).unwrap();
      let index_buffer = get_sub_buffer(index_buffer.as_slice(), index.1);
      let index_buffer = bytemuck::cast_slice(index_buffer);

      let source = BottomLevelAccelerationStructureBuildSource {
        geometry: BottomLevelAccelerationStructureBuildBuffer::Triangles {
          positions: position_buffer.to_vec(), // todo, avoid vec
          indices: index_buffer.to_vec(),
        },
        flags: 0, // todo check
      };
      BlasInstance {
        handle: acc_sys.create_bottom_level_acceleration_structure(&[source]),
        sys: acc_sys.clone(),
      }
    }
  })
}

pub fn scene_model_to_tlas_instance(
  acc_sys: Box<dyn GPUAccelerationStructureSystemProvider>,
) -> impl ReactiveQuery<Key = EntityHandle<SceneModelEntity>, Value = (BlasInstance, Mat4<f32>)> {
  EmptyQuery::default()
}

pub fn scene_to_tlas(
  acc_sys: Box<dyn GPUAccelerationStructureSystemProvider>,
) -> impl ReactiveQuery<Key = EntityHandle<SceneModelEntity>, Value = TlasInstance> {
  EmptyQuery::default()
}
