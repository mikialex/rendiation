use crate::*;

pub type UniformBufferCollectionRaw<K, T> = FastHashMap<K, UniformBufferDataView<T>>;
pub type UniformBufferCollection<K, T> = Arc<RwLock<FastHashMap<K, UniformBufferDataView<T>>>>;

pub trait DataChangeGPUExt<K: LinearIdentified + CKey> {
  fn update_uniforms<U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  );

  fn update_uniform_array<U: Std140 + ShaderSizedValueNodeType + Default, const N: usize>(
    &self,
    uniforms: &UniformArray<U, N>,
    offset: usize,
    gpu: &GPU,
  );

  fn update_storage_array<U: Std430 + ShaderSizedValueNodeType + Default>(
    &self,
    storage: &mut CommonStorageBufferImpl<U>,
    field_offset: usize,
  );
}

// I'm so sad
pub trait DataChangeGPUExtForUseResult<K: LinearIdentified + CKey> {
  fn update_uniforms<U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  );
  fn update_uniform_array<U: Std140 + ShaderSizedValueNodeType + Default, const N: usize>(
    &self,
    uniforms: &UniformArray<U, N>,
    field_offset: usize,
    gpu: &GPU,
  );
  fn update_storage_array<U: Std430 + ShaderSizedValueNodeType + Default>(
    &self,
    storage: &mut CommonStorageBufferImpl<U>,
    field_offset: usize,
  );
}

impl<K: LinearIdentified + CKey, T: DataChangeGPUExt<K>> DataChangeGPUExtForUseResult<K>
  for UseResult<T>
{
  fn update_uniforms<U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  ) {
    let r = match self {
      UseResult::SpawnStageReady(r) => r,
      UseResult::ResolveStageReady(r) => r,
      _ => return,
    };
    r.update_uniforms(uniforms, offset, gpu);
  }

  fn update_uniform_array<U: Std140 + ShaderSizedValueNodeType + Default, const N: usize>(
    &self,
    uniforms: &UniformArray<U, N>,
    field_offset: usize,
    gpu: &GPU,
  ) {
    let r = match self {
      UseResult::SpawnStageReady(r) => r,
      UseResult::ResolveStageReady(r) => r,
      _ => return,
    };
    r.update_uniform_array(uniforms, field_offset, gpu);
  }

  fn update_storage_array<U: Std430 + ShaderSizedValueNodeType + Default>(
    &self,
    storage: &mut CommonStorageBufferImpl<U>,
    field_offset: usize,
  ) {
    let r = match self {
      UseResult::SpawnStageReady(r) => r,
      UseResult::ResolveStageReady(r) => r,
      _ => return,
    };
    r.update_storage_array(storage, field_offset);
  }
}

impl<K, T, X> DataChangeGPUExt<K> for X
where
  T: Pod,
  K: LinearIdentified + CKey,
  X: DataChanges<Key = K, Value = T>,
{
  fn update_uniforms<U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  ) {
    if self.has_change() {
      let mut uniform = uniforms.write();
      for id in self.iter_removed() {
        uniform.remove(&id);
      }

      for (id, value) in self.iter_update_or_insert() {
        let buffer = uniform
          .entry(id)
          .or_insert_with(|| UniformBufferDataView::create_default(&gpu.device));
        // todo, here we should do sophisticated optimization to merge the adjacent writes.
        buffer.write_at(&gpu.queue, &value, offset as u64);
      }
    }
  }

  fn update_uniform_array<U: Std140 + Default, const N: usize>(
    &self,
    uniforms: &UniformArray<U, N>,
    field_offset: usize,
    gpu: &GPU,
  ) {
    if self.has_change() {
      for (id, value) in self.iter_update_or_insert() {
        let offset = id.alloc_index() as usize * std::mem::size_of::<U>() + field_offset;

        // here we should do sophisticated optimization to merge the adjacent writes.
        uniforms.write_at(&gpu.queue, &value, offset as u64);
      }
    }
  }

  fn update_storage_array<U: Std430 + ShaderSizedValueNodeType + Default>(
    &self,
    storage: &mut CommonStorageBufferImpl<U>,
    field_offset: usize,
  ) {
    if self.has_change() {
      for (id, value) in self.iter_update_or_insert() {
        unsafe {
          storage
            .set_value_sub_bytes(id.alloc_index(), field_offset, bytes_of(&value))
            .unwrap();
        }
      }
    }
  }
}
