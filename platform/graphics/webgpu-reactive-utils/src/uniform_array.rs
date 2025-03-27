use crate::*;

pub type UniformArray<T, const N: usize> = UniformBufferDataView<Shader140Array<T, N>>;

pub type UniformArrayUpdateContainer<T, const N: usize> = MultiUpdateContainer<UniformArray<T, N>>;

pub struct QueryBasedUniformArrayUpdate<T> {
  field_offset: u32,
  upstream: T,
  gpu_ctx: GPU,
}

pub trait UniformArrayQueryUpdateExt: Sized {
  fn into_query_update_uniform_array(
    self,
    field_offset: usize,
    gpu_ctx: &GPU,
  ) -> QueryBasedUniformArrayUpdate<Self>;
}
impl<T> UniformArrayQueryUpdateExt for T
where
  T: ReactiveQuery,
{
  fn into_query_update_uniform_array(
    self,
    field_offset: usize,
    gpu_ctx: &GPU,
  ) -> QueryBasedUniformArrayUpdate<Self> {
    QueryBasedUniformArrayUpdate {
      field_offset: field_offset as u32,
      upstream: self,
      gpu_ctx: gpu_ctx.clone(),
    }
  }
}

impl<T, C, const N: usize> QueryBasedUpdate<UniformArray<T, N>> for QueryBasedUniformArrayUpdate<C>
where
  T: Std140 + Default,
  C: ReactiveQuery,
  C::Key: LinearIdentified,
  C::Value: Pod,
{
  fn update_target(&mut self, target: &mut UniformArray<T, N>, cx: &mut Context) {
    let (changes, _) = self.upstream.describe(cx).resolve_kept();
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

pub trait UniformArrayQueryResultCtxExt {
  fn take_uniform_array_buffer<T: Std140, const N: usize>(
    &mut self,
    token: QueryToken,
  ) -> Option<UniformArray<T, N>>;
}

impl UniformArrayQueryResultCtxExt for QueryResultCtx {
  fn take_uniform_array_buffer<T: Std140, const N: usize>(
    &mut self,
    token: QueryToken,
  ) -> Option<UniformArray<T, N>> {
    self
      .take_multi_updater_updated::<UniformArray<T, N>>(token)?
      .target
      .clone()
      .into()
  }
}
