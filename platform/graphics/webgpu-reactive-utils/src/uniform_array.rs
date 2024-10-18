use crate::*;

pub type UniformArray<T, const N: usize> = UniformBufferDataView<Shader140Array<T, N>>;

pub type UniformArrayUpdateContainer<T> = MultiUpdateContainer<UniformArray<T, 8>>;

pub struct UniformArrayCollectionUpdate<T> {
  field_offset: u32,
  upstream: T,
  gpu_ctx: GPU,
}

pub trait UniformArrayCollectionUpdateExt: Sized {
  fn into_uniform_array_collection_update(
    self,
    field_offset: usize,
    gpu_ctx: &GPU,
  ) -> UniformArrayCollectionUpdate<Self>;
}
impl<T> UniformArrayCollectionUpdateExt for T
where
  T: ReactiveCollection,
{
  fn into_uniform_array_collection_update(
    self,
    field_offset: usize,
    gpu_ctx: &GPU,
  ) -> UniformArrayCollectionUpdate<Self> {
    UniformArrayCollectionUpdate {
      field_offset: field_offset as u32,
      upstream: self,
      gpu_ctx: gpu_ctx.clone(),
    }
  }
}

impl<T, C> CollectionUpdate<UniformArray<T, 8>> for UniformArrayCollectionUpdate<C>
where
  T: Std140 + Default,
  C: ReactiveCollection,
  C::Key: LinearIdentified,
  C::Value: Pod,
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
