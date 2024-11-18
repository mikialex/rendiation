use crate::*;

fn get_sub_buffer(buffer: &[u8], range: Option<BufferViewRange>) -> &[u8] {
  if let Some(range) = range {
    buffer.get(range.into_range(buffer.len())).unwrap()
  } else {
    buffer
  }
}

#[derive(Clone)]
pub struct BlasInstance {
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

pub fn attribute_mesh_to_blas(
  acc_sys: Box<dyn GPUAccelerationStructureSystemProvider>,
) -> impl ReactiveQuery<Key = EntityHandle<AttributesMeshEntity>, Value = BlasInstance> {
  let PositionRelatedAttributeMeshQuery {
    indexed,
    none_indexed,
  } = attribute_mesh_position_query();

  let acc_sys_ = acc_sys.clone();
  let none_indexed = none_indexed.collective_execute_map_by(move || {
    let acc_sys = acc_sys_.clone();
    let buffer_accessor = global_entity_component_of::<BufferEntityData>().read();
    move |_, position| {
      let position_buffer = buffer_accessor.get(position.0.unwrap()).unwrap();
      let position_buffer = get_sub_buffer(position_buffer.as_slice(), position.1);
      let position_buffer = bytemuck::cast_slice(position_buffer);

      let source = BottomLevelAccelerationStructureBuildSource {
        geometry: BottomLevelAccelerationStructureBuildBuffer::Triangles {
          positions: position_buffer.to_vec(), // todo, avoid vec
          indices: todo!(),
        },
        flags: 0, // todo check
      };
      BlasInstance {
        handle: acc_sys.create_bottom_level_acceleration_structure(&[source]),
        sys: acc_sys.clone(),
      }
    }
  });

  indexed
    .collective_execute_map_by(move || {
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
    .collective_select(none_indexed)
}

pub fn scene_model_to_tlas_instance(
  acc_sys: Box<dyn GPUAccelerationStructureSystemProvider>,
) -> impl ReactiveQuery<Key = EntityHandle<SceneModelEntity>, Value = (BlasInstance, Mat4<f32>)> {
  // todo, this should register into registry
  let scene_node_world_mat = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelRefNode>());

  attribute_mesh_to_blas(acc_sys)
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<StandardModelRefAttributesMeshEntity>())
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelStdModelRenderPayload>())
    .collective_zip(scene_node_world_mat)
}

pub fn scene_to_tlas(
  acc_sys: Box<dyn GPUAccelerationStructureSystemProvider>,
) -> impl ReactiveQuery<Key = EntityHandle<SceneModelEntity>, Value = TlasInstance> {
  global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>();
  //
  EmptyQuery::default()
}
