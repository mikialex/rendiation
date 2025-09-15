use crate::*;

#[derive(Clone)]
pub struct TlASInstance {
  pub tlas_handle: TlasHandle,
}
impl std::fmt::Debug for TlASInstance {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("TlASInstance").finish()
  }
}
impl PartialEq for TlASInstance {
  fn eq(&self, _other: &Self) -> bool {
    false
  }
}

fn get_sub_buffer(buffer: &[u8], range: Option<BufferViewRange>) -> &[u8] {
  if let Some(range) = range {
    buffer.get(range.into_range(buffer.len())).unwrap()
  } else {
    buffer
  }
}

#[derive(Clone)]
pub struct BlasInstance {
  inner: Arc<BlasInstanceInternal>,
}

impl BlasInstance {
  pub fn new(handle: BlasHandle, sys: Box<dyn GPUAccelerationStructureSystemProvider>) -> Self {
    Self {
      inner: Arc::new(BlasInstanceInternal { handle, sys }),
    }
  }
  pub fn handle(&self) -> BlasHandle {
    self.inner.handle
  }
}

struct BlasInstanceInternal {
  handle: BlasHandle,
  sys: Box<dyn GPUAccelerationStructureSystemProvider>,
}

impl std::fmt::Debug for BlasInstance {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("BlasInstance")
      .field("handle", &self.inner.handle)
      .finish()
  }
}

impl PartialEq for BlasInstance {
  fn eq(&self, other: &Self) -> bool {
    self.inner.handle == other.inner.handle
  }
}

impl Drop for BlasInstanceInternal {
  fn drop(&mut self) {
    self
      .sys
      .delete_bottom_level_acceleration_structure(self.handle);
  }
}

// Key: AttributesMeshEntity
pub fn use_attribute_mesh_to_blas(
  cx: &mut QueryGPUHookCx,
  acc_sys: &Box<dyn GPUAccelerationStructureSystemProvider>,
) -> UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = BlasInstance>> {
  let (position_scopes1, position_scopes2) = use_attribute_mesh_position_query(cx).fork();

  let acc_sys_ = acc_sys.clone();

  let none_indexed = position_scopes1
    .map(|v| v.none_indexed)
    .use_dual_query_execute_map(cx, move || {
      let acc_sys = acc_sys_;
      let buffer_accessor = get_db_view::<BufferEntityData>();
      move |_, position| {
        let position_buffer = buffer_accessor.access(&position.0.unwrap()).unwrap();
        let position_buffer = get_sub_buffer(position_buffer.as_slice(), position.1);
        let position_buffer = bytemuck::cast_slice(position_buffer);

        let source = BottomLevelAccelerationStructureBuildSource {
          geometry: BottomLevelAccelerationStructureBuildBuffer::Triangles {
            positions: position_buffer.to_vec(), // todo, avoid vec
            indices: None,
          },
          flags: GEOMETRY_FLAG_OPAQUE,
        };
        BlasInstance::new(
          acc_sys.create_bottom_level_acceleration_structure(&[source]),
          acc_sys.clone(),
        )
      }
    });

  let acc_sys__ = acc_sys.clone();

  let indexed = position_scopes2
    .map(|v| v.indexed)
    .use_dual_query_execute_map(cx, move || {
      let acc_sys = acc_sys__;
      let buffer_accessor = get_db_view::<BufferEntityData>();
      move |_, (position, (index_buffer, index_range, index_count))| {
        let position_buffer = buffer_accessor.access(&position.0.unwrap()).unwrap();
        let position_buffer = get_sub_buffer(position_buffer.as_slice(), position.1);
        let position_buffer = bytemuck::cast_slice(position_buffer);

        let index_buffer = buffer_accessor.access(&index_buffer).unwrap();
        let index = get_sub_buffer(index_buffer.as_slice(), index_range);

        let count = index_count as usize;
        let index = if index.len() / count == 2 {
          let index: &[u16] = cast_slice(index);
          index.iter().map(|i| *i as u32).collect()
        } else if index.len() / count == 4 {
          let index: &[u32] = cast_slice(index);
          index.to_vec()
        } else {
          unreachable!("index count must be 2 or 4")
        };

        let source = BottomLevelAccelerationStructureBuildSource {
          geometry: BottomLevelAccelerationStructureBuildBuffer::Triangles {
            positions: position_buffer.to_vec(), // todo, avoid vec
            indices: Some(index),
          },
          flags: GEOMETRY_FLAG_OPAQUE,
        };
        BlasInstance::new(
          acc_sys.create_bottom_level_acceleration_structure(&[source]),
          acc_sys.clone(),
        )
      }
    });

  indexed.dual_query_select(none_indexed)
}

pub fn use_scene_model_to_blas_instance(
  cx: &mut QueryGPUHookCx,
  acc_sys: &Box<dyn GPUAccelerationStructureSystemProvider>,
  // SceneModelEntity
) -> UseResult<impl DualQueryLike<Key = RawEntityHandle, Value = (BlasInstance, Mat4<f64>)>> {
  let scene_model_world_matrix = cx.use_shared_dual_query(GlobalSceneModelWorldMatrix);

  let std_model_ref_mesh = cx.use_db_rev_ref_tri_view::<StandardModelRefAttributesMeshEntity>();
  let std_model_render_payload = cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>();

  use_attribute_mesh_to_blas(cx, acc_sys)
    .fanout(std_model_ref_mesh, cx)
    .fanout(std_model_render_payload, cx)
    .dual_query_intersect(scene_model_world_matrix)
}

pub fn use_scene_to_tlas(
  cx: &mut QueryGPUHookCx,
  acc_sys: &Box<dyn GPUAccelerationStructureSystemProvider>,
  // SceneEntity
) -> Option<impl Query<Key = RawEntityHandle, Value = TlASInstance>> {
  let tlas_store = cx.use_shared_hash_map::<RawEntityHandle, TlASInstance>();

  let scene_sm = cx
    .use_db_rev_ref_tri_view::<SceneModelBelongsToScene>()
    .use_assure_result(cx);

  if let Some(blas_source) = use_scene_model_to_blas_instance(cx, acc_sys)
    .use_assure_result(cx)
    .if_ready()
  {
    let mut tlas = tlas_store.write();

    let mut regenerate_scene = FastHashSet::<RawEntityHandle>::default();

    let TriQuery {
      base:
        DualQuery {
          view: current_sm_acc_scene,
          delta: scene_ref_sm_change,
        },
      rev_many_view,
    } = scene_sm.expect_resolve_stage();

    for (_, change) in scene_ref_sm_change.iter_key_value() {
      if let Some(new_scene) = change.new_value() {
        regenerate_scene.insert(*new_scene);
      }
      if let Some(new_scene) = change.old_value() {
        regenerate_scene.insert(*new_scene);
      }
    }

    let (current_sm_blas, sm_blas_change) = blas_source.view_delta();
    for (k, _) in sm_blas_change.iter_key_value() {
      if let Some(scene) = current_sm_acc_scene.access(&k) {
        regenerate_scene.insert(scene);
      }
    }

    for scene in regenerate_scene.drain() {
      if let Some(tlas) = tlas.remove(&scene) {
        acc_sys.delete_top_level_acceleration_structure(tlas.tlas_handle);
      }
      let source = rev_many_view
        .access_multi(&scene)
        .unwrap()
        .filter_map(|sm| {
          current_sm_blas.access(&sm).map(|(blas, transform)| {
            TopLevelAccelerationStructureSourceInstance {
              transform: transform.into_f32(),
              instance_custom_index: sm.alloc_index(),
              mask: u32::MAX,
              instance_shader_binding_table_record_offset: sm.alloc_index()
                * GLOBAL_TLAS_MAX_RAY_STRIDE,
              flags: 0,
              acceleration_structure_handle: blas.handle(),
            }
          })
        })
        .collect::<Vec<_>>();
      let new_tlas = acc_sys.create_top_level_acceleration_structure(source.as_slice());
      tlas.insert(
        scene,
        TlASInstance {
          tlas_handle: new_tlas,
        },
      );
    }

    drop(tlas);

    Some(tlas_store.make_read_holder())
  } else {
    None
  }
}

pub const GLOBAL_TLAS_MAX_RAY_STRIDE: u32 = 4;
