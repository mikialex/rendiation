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
  handle: BlasHandle,
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
          indices: None,
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
            indices: Some(index_buffer.to_vec()),
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
) -> impl ReactiveQuery<Key = EntityHandle<SceneEntity>, Value = TlasHandle> {
  SceneTlasMaintainer {
    acc_sys: acc_sys.clone(),
    source: scene_model_to_tlas_instance(acc_sys).into_boxed(),
    scene_sm: Box::new(global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>()),
    tlas: Default::default(),
  }
}

struct SceneTlasMaintainer {
  acc_sys: Box<dyn GPUAccelerationStructureSystemProvider>,
  source: BoxedDynReactiveQuery<EntityHandle<SceneModelEntity>, (BlasInstance, Mat4<f32>)>,
  scene_sm:
    BoxedDynReactiveOneToManyRelation<EntityHandle<SceneEntity>, EntityHandle<SceneModelEntity>>,
  tlas: Arc<RwLock<FastHashMap<EntityHandle<SceneEntity>, TlasHandle>>>,
}

impl ReactiveQuery for SceneTlasMaintainer {
  type Key = EntityHandle<SceneEntity>;
  type Value = TlasHandle;
  type Changes = impl Query<Key = Self::Key, Value = ValueChange<Self::Value>>;
  type View = impl Query<Key = Self::Key, Value = Self::Value>;

  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let mut tlas = self.tlas.write();

    let mut mutations =
      FastHashMap::<EntityHandle<SceneEntity>, ValueChange<TlasHandle>>::default();
    let mut mutator = QueryMutationCollector {
      delta: &mut mutations,
      target: tlas.deref_mut(),
    };

    let mut regenerate_scene = FastHashSet::<EntityHandle<SceneEntity>>::default();
    let (scene_ref_sm_change, current_sm_acc_scene, current_scene_ref_sm) =
      self.scene_sm.poll_changes_with_inv_dyn(cx);
    for (_, change) in scene_ref_sm_change.iter_key_value() {
      if let Some(new_scene) = change.new_value() {
        regenerate_scene.insert(*new_scene);
      }
      if let Some(new_scene) = change.old_value() {
        regenerate_scene.insert(*new_scene);
      }
    }

    let (sm_blas_change, current_sm_blas) = self.source.poll_changes(cx);
    for (k, _) in sm_blas_change.iter_key_value() {
      if let Some(scene) = current_sm_acc_scene.access(&k) {
        regenerate_scene.insert(scene);
      }
    }

    for scene in regenerate_scene.drain() {
      if let Some(tlas) = mutator.remove(scene) {
        self.acc_sys.delete_top_level_acceleration_structure(tlas);
      }
      let source = current_scene_ref_sm
        .access_multi(&scene)
        .unwrap()
        .filter_map(|sm| {
          current_sm_blas.access(&sm).map(|(blas, transform)| {
            TopLevelAccelerationStructureSourceInstance {
              transform,
              instance_custom_index: sm.alloc_index(),
              mask: 0, // todo check
              instance_shader_binding_table_record_offset: sm.alloc_index(),
              flags: 0, // todo check
              acceleration_structure_handle: blas.handle,
            }
          })
        })
        .collect::<Vec<_>>();
      let new_tlas = self
        .acc_sys
        .create_top_level_acceleration_structure(source.as_slice());
      mutator.set_value(scene, new_tlas);
    }

    drop(tlas);

    (mutations, self.tlas.make_read_holder())
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.source.request(request);
    self.scene_sm.request(request);
  }
}
