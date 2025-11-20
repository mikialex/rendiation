use crate::*;

pub struct PersistSceneModelListBuffer {
  buffer: Option<PersistSceneModelListBufferWithLength>,
  host: Vec<RawEntityHandle>,
  mapping: FastHashMap<RawEntityHandle, usize>,
}

impl Default for PersistSceneModelListBuffer {
  fn default() -> Self {
    Self::with_capacity(1024)
  }
}

pub struct PersistSceneModelListBufferMutation {
  mapping_change: FastHashMap<usize, u32>,
  len_before_updates: usize,
  new_len: usize,
}

impl PersistSceneModelListBufferMutation {
  pub fn into_sparse_update(self) -> Option<SparseBufferWritesSource> {
    let change_count = self.mapping_change.len();
    if change_count == 0 {
      return None;
    }
    let change_count = change_count + 1;
    let byte_change_capacity = change_count * 4;
    let mut updates = SparseBufferWritesSource::with_capacity(byte_change_capacity, change_count);

    for (idx, val) in self.mapping_change {
      updates.collect_write(bytes_of(&val), (idx + 1) as u64 * 4);
    }

    if self.len_before_updates != self.new_len {
      updates.collect_write(bytes_of(&(self.new_len as u32)), 0);
    }

    updates.into()
  }
}

impl PersistSceneModelListBuffer {
  pub fn create_batch(&self) -> Option<DeviceSceneModelRenderSubBatch> {
    DeviceSceneModelRenderSubBatch {
      scene_models: Box::new(self.buffer.clone().unwrap()),
      impl_select_id: unsafe { EntityHandle::from_raw(*self.host.first()?) }, // maybe empty
    }
    .into()
  }
  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      buffer: None,
      host: Vec::with_capacity(capacity),
      mapping: FastHashMap::with_capacity_and_hasher(capacity, FastHasherBuilder::default()),
    }
  }

  pub fn create_mutation(&self) -> PersistSceneModelListBufferMutation {
    PersistSceneModelListBufferMutation {
      mapping_change: Default::default(),
      len_before_updates: self.host.len(),
      new_len: self.host.len(),
    }
  }

  pub fn insert(
    &mut self,
    sm_handle: RawEntityHandle,
    mutations: &mut PersistSceneModelListBufferMutation,
  ) {
    self.host.push(sm_handle);
    self.mapping.insert(sm_handle, self.host.len() - 1);
    mutations
      .mapping_change
      .insert(self.host.len() - 1, sm_handle.alloc_index());
    mutations.new_len = self.host.len();
  }

  pub fn remove(
    &mut self,
    sm_handle: RawEntityHandle,
    mutations: &mut PersistSceneModelListBufferMutation,
  ) {
    let idx = self.mapping.remove(&sm_handle).unwrap();
    if let Some(tail_item) = self.host.last().cloned() {
      self.host.swap_remove(idx);
      self.mapping.insert(tail_item, idx);

      mutations.mapping_change.remove(&self.host.len());
      mutations
        .mapping_change
        .insert(idx, tail_item.alloc_index());
    }

    mutations.new_len = self.host.len();
  }

  pub fn update_gpu(
    &mut self,
    alloc: &dyn AbstractStorageAllocator,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
    updates: &SparseBufferWritesSource,
  ) {
    let new_capacity_required = self.host.len() + 1;
    let new_bytes_required = new_capacity_required * 4;

    if let Some(buffer) = &self.buffer {
      if buffer.buffer.item_count() < new_capacity_required as u32 {
        self.buffer = None;
      }
    }

    let buffer = self
      .buffer
      .get_or_insert_with(|| PersistSceneModelListBufferWithLength {
        buffer: alloc.allocate_readonly(
          new_bytes_required as u64,
          &gpu.device,
          Some("PersistSceneModelListBuffer"),
        ),
      });

    updates.write_abstract(gpu, encoder, &buffer.buffer);
  }
}

/// the [0] store the real length
#[derive(Clone)]
struct PersistSceneModelListBufferWithLength {
  buffer: AbstractReadonlyStorageBuffer<[u32]>,
}
impl ComputeComponentIO<u32> for PersistSceneModelListBufferWithLength {}
impl ShaderHashProvider for PersistSceneModelListBufferWithLength {
  shader_hash_type_id! {}
}
impl ComputeComponent<Node<u32>> for PersistSceneModelListBufferWithLength {
  fn work_size(&self) -> Option<u32> {
    None
  }

  fn result_size(&self) -> u32 {
    self.buffer.item_count() - 1
  }
  fn clone_boxed(&self) -> Box<dyn ComputeComponent<Node<u32>>> {
    Box::new(self.clone())
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<u32>>> {
    struct Invocation {
      buffer: ShaderReadonlyPtrOf<[u32]>,
    }

    impl DeviceInvocation<Node<u32>> for Invocation {
      fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<u32>, Node<bool>) {
        let idx = logic_global_id.x();
        let access_idx = idx + val(1);

        let array_len = self.buffer.array_length();
        let r = access_idx.less_than(array_len);

        let result = r.select_branched(|| self.buffer.index(access_idx).load(), || val(0_u32));

        (result, r)
      }

      fn invocation_size(&self) -> Node<Vec3<u32>> {
        (self.buffer.index(0).load(), val(0), val(0)).into()
      }
    }

    Box::new(Invocation {
      buffer: builder.bind_by(&self.buffer),
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.buffer);
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }
}
