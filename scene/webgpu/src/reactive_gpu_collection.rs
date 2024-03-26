use crate::*;

pub trait AllocIdCollectionGPUExt<K: 'static> {
  // todo, remove parallel?
  fn collective_execute_gpu_map<V>(
    self,
    gpu: ResourceGPUCtx,
    mapper: impl Fn(&K, &ResourceGPUCtx) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollectionSelfContained<AllocIdx<K>, V>
  where
    V: CValue;

  fn collective_create_uniforms_by_key<V>(
    self,
    gpu: ResourceGPUCtx,
    mapper: impl Fn(&K) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollectionSelfContained<AllocIdx<K>, UniformBufferDataView<V>>
  where
    V: Std140 + Send + Sync;
}

impl<K, T> AllocIdCollectionGPUExt<K> for T
where
  T: ReactiveCollection<AllocIdx<K>, AnyChanging>,
  K: IncrementalBase,
{
  fn collective_execute_gpu_map<V>(
    self,
    gpu: ResourceGPUCtx,
    mapper: impl Fn(&K, &ResourceGPUCtx) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollectionSelfContained<AllocIdx<K>, V>
  where
    V: CValue,
  {
    let gpu = gpu.clone();
    self.collective_execute_map_by(move || {
      let gpu = gpu.clone();
      let creator = storage_of::<K>().create_key_mapper(move |m, _| mapper(m, &gpu));
      move |k, _| creator(*k)
    })
  }

  fn collective_create_uniforms_by_key<V>(
    self,
    gpu: ResourceGPUCtx,
    mapper: impl Fn(&K) -> V + 'static + Send + Sync + Copy,
  ) -> impl ReactiveCollectionSelfContained<AllocIdx<K>, UniformBufferDataView<V>>
  where
    V: Std140 + Send + Sync,
  {
    self.collective_execute_gpu_map(gpu, move |k, gpu| {
      let uniform = mapper(k);
      create_uniform(uniform, &gpu.device)
    })
  }
}

pub trait CollectionGPUExt<K: CKey, V: CValue> {
  fn collective_create_uniforms(
    self,
    gpu: ResourceGPUCtx,
  ) -> impl ReactiveCollection<K, UniformBufferDataView<V>>
  where
    V: Std140 + Send + Sync;
}
impl<K: CKey, V: CValue, T> CollectionGPUExt<K, V> for T
where
  T: ReactiveCollection<K, V>,
{
  fn collective_create_uniforms(
    self,
    gpu: ResourceGPUCtx,
  ) -> impl ReactiveCollection<K, UniformBufferDataView<V>>
  where
    V: Std140 + Send + Sync,
  {
    let gpu = gpu.clone();
    self.collective_execute_map_by(move || {
      let gpu = gpu.clone();
      move |_, uniform| create_uniform(uniform, &gpu.device)
    })
  }
}
