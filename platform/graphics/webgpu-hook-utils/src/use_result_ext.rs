use std::future::Future;

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
  fn update_storage_array<U>(
    self,
    cx: &mut QueryGPUHookCx,
    storage: &mut SparseUpdateStorageBuffer<U>,
    field_offset: usize,
  ) where
    U: Std430 + ShaderSizedValueNodeType + Default;

  fn update_storage_array_with_host<U>(
    self,
    cx: &mut QueryGPUHookCx,
    storage: &mut SparseUpdateStorageWithHostBuffer<U>,
    field_offset: usize,
  ) where
    U: Std430 + ShaderSizedValueNodeType + Default,
    K: LinearIdentified + CKey;

  fn update_storage_array_raw<U>(
    self,
    cx: &mut QueryGPUHookCx,
    collector: Option<&mut SparseUpdateCollector>,
    field_offset: usize,
  ) where
    U: Std430 + ShaderSizedValueNodeType + Default,
    K: LinearIdentified + CKey;
}

impl<K, T> DataChangeGPUExtForUseResult<K> for UseResult<T>
where
  K: LinearIdentified + CKey,
  T: DataChangeGPUExt<K>,
  T: DataChanges<Key = K>,
  T: 'static,
  T::Value: Pod,
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

  fn update_storage_array<U>(
    self,
    cx: &mut QueryGPUHookCx,
    storage: &mut SparseUpdateStorageBuffer<U>,
    field_offset: usize,
  ) where
    U: Std430 + ShaderSizedValueNodeType + Default,
    K: LinearIdentified + CKey,
  {
    self.update_storage_array_raw::<U>(cx, storage.collector.as_mut(), field_offset);
  }

  fn update_storage_array_with_host<U>(
    self,
    cx: &mut QueryGPUHookCx,
    storage: &mut SparseUpdateStorageWithHostBuffer<U>,
    field_offset: usize,
  ) where
    U: Std430 + ShaderSizedValueNodeType + Default,
    K: LinearIdentified + CKey,
  {
    self.update_storage_array_raw::<U>(cx, storage.collector.as_mut(), field_offset);
  }

  fn update_storage_array_raw<U>(
    self,
    cx: &mut QueryGPUHookCx,
    collector: Option<&mut SparseUpdateCollector>,
    field_offset: usize,
  ) where
    U: Std430 + ShaderSizedValueNodeType + Default,
    K: LinearIdentified + CKey,
  {
    #[cfg(debug_assertions)]
    let (cx, has_change) = cx.use_plain_state_default::<bool>();
    let r = match self {
      UseResult::SpawnStageReady(r) => {
        #[cfg(debug_assertions)]
        {
          *has_change = true;
        }

        pin_box_in_frame(futures::future::ready(r))
          as std::pin::Pin<FrameBox<dyn Future<Output = T> + Send>>
      }
      UseResult::SpawnStageFuture(f) => {
        #[cfg(debug_assertions)]
        {
          *has_change = true;
        }

        f
      }
      UseResult::ResolveStageReady(_) => {
        #[cfg(debug_assertions)]
        {
          if !*has_change {
            panic!("storage array update must prepared in spawn stage")
          }
          *has_change = false;
        }

        return;
      }
      _ => return,
    };

    if let GPUQueryHookStage::Update { spawner, .. } = cx.stage {
      let spawner = spawner.clone();
      let collector = collector.unwrap();

      let f = async move {
        let r = r.await;
        if r.has_change() {
          spawner
            .spawn_task(move || {
              let mut write_src = SparseBufferWritesSource::default();
              let item_size = std::mem::size_of::<U>() as u32;
              // todo, avoid resize
              r.iter_update_or_insert().for_each(|(id, value)| {
                let offset = item_size * id.alloc_index() + field_offset as u32;
                write_src.collect_write(bytes_of(&value), offset as u64);
              });
              write_src
            })
            .await
        } else {
          SparseBufferWritesSource::default()
        }
      };

      collector.push(pin_box_in_frame(f));
    }
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

      if uniform.capacity() > uniform.len() * 2 {
        uniform.shrink_to_fit();
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
}
