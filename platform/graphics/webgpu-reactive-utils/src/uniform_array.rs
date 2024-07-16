use crate::*;

pub type UniformArray<T, const N: usize> = UniformBufferDataView<Shader140Array<T, N>>;

pub type UniformArrayUpdateContainer<T> = MultiUpdateContainer<UniformArray<T, 8>>;

pub struct UniformArrayCollectionUpdate<T, K, V> {
  field_offset: u32,
  upstream: T,
  phantom: PhantomData<(K, V)>,
  gpu_ctx: GPU,
}

pub trait UniformArrayCollectionUpdateExt<K, V>: Sized {
  fn into_uniform_array_collection_update(
    self,
    field_offset: usize,
    gpu_ctx: &GPU,
  ) -> UniformArrayCollectionUpdate<Self, K, V>;
}
impl<K, V, T> UniformArrayCollectionUpdateExt<K, V> for T
where
  T: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  fn into_uniform_array_collection_update(
    self,
    field_offset: usize,
    gpu_ctx: &GPU,
  ) -> UniformArrayCollectionUpdate<Self, K, V> {
    UniformArrayCollectionUpdate {
      field_offset: field_offset as u32,
      upstream: self,
      phantom: PhantomData,
      gpu_ctx: gpu_ctx.clone(),
    }
  }
}

impl<T, C, K, V> CollectionUpdate<UniformArray<T, 8>> for UniformArrayCollectionUpdate<C, K, V>
where
  T: Std140 + Default,
  V: CValue + Pod,
  K: CKey + LinearIdentified,
  C: ReactiveCollection<K, V>,
{
  fn update_target(&mut self, target: &mut UniformArray<T, 8>, cx: &mut Context) {
    let (changes, _) = self.upstream.poll_changes(cx);
    for (k, v) in changes.iter_key_value() {
      let index = k.alloc_index();

      match v {
        ValueChange::Delta(v, _) => {
          let offset = index as usize * std::mem::size_of::<T>() + self.field_offset as usize;

          // here we should do sophisticated optimization to merge the adjacent writes.
          target.write_at(&self.gpu_ctx.queue, &v, offset as u64);
        }
        ValueChange::Remove(_) => {
          // we could do clear in debug mode
        }
      }
    }
  }
}
