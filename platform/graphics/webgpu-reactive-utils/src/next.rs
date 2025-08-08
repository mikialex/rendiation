use crate::*;

pub type UniformBufferCollectionRaw<K, T> = FastHashMap<K, UniformBufferDataView<T>>;
pub type UniformBufferCollection<K, T> = Arc<RwLock<FastHashMap<K, UniformBufferDataView<T>>>>;

pub trait DataChangeGPUExt {
  fn update_uniforms<K: LinearIdentification + CKey, U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  );

  fn update_storage_array<U: Std430 + Default>(
    &self,
    storage: &mut CommonStorageBufferImpl<U>,
    field_offset: usize,
  );
}

pub trait DataChangeGPUExtOptionHelper {
  fn update_uniforms<K: LinearIdentification + CKey, U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  );

  fn update_storage_array<U: Std430 + Default>(
    &self,
    storage: &mut CommonStorageBufferImpl<U>,
    field_offset: usize,
  );
}
impl<T: DataChangeGPUExt> DataChangeGPUExtOptionHelper for Option<T> {
  fn update_uniforms<K: LinearIdentification + CKey, U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  ) {
    if let Some(t) = self {
      t.update_uniforms(uniforms, offset, gpu);
    }
  }

  fn update_storage_array<U: Std430 + Default>(
    &self,
    storage: &mut CommonStorageBufferImpl<U>,
    field_offset: usize,
  ) {
    if let Some(t) = self {
      t.update_storage_array(storage, field_offset);
    }
  }
}

impl<T, X> DataChangeGPUExt for X
where
  T: Pod,
  X: DataChanges<Key = u32, Value = T>,
{
  fn update_uniforms<K: LinearIdentification + CKey, U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  ) {
    if self.has_change() {
      let mut uniform = uniforms.write();
      for id in self.iter_removed() {
        uniform.remove(&K::from_alloc_index(id));
      }

      for (id, value) in self.iter_update_or_insert() {
        let buffer = uniform
          .entry(K::from_alloc_index(id))
          .or_insert_with(|| UniformBufferDataView::create_default(&gpu.device));
        // todo, here we should do sophisticated optimization to merge the adjacent writes.
        buffer.write_at(&gpu.queue, &value, offset as u64);
      }
    }
  }

  fn update_storage_array<U: Std430 + Default>(
    &self,
    storage: &mut CommonStorageBufferImpl<U>,
    field_offset: usize,
  ) {
    if self.has_change() {
      for (id, value) in self.iter_update_or_insert() {
        unsafe {
          storage
            .set_value_sub_bytes(id, field_offset, bytes_of(&value))
            .unwrap();
        }
      }
    }

    //
  }
}
