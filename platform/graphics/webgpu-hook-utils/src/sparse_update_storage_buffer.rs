use std::{future::Future, pin::Pin};

use database::EntitySemantic;
use futures::future::join_all;

use crate::*;

type SparseStorageBufferRaw<T> =
  CustomGrowBehaviorMaintainer<ResizableGPUBuffer<AbstractReadonlyStorageBuffer<[T]>>>;

pub struct SparseUpdateStorageBuffer<T> {
  buffer: SparseStorageBufferRaw<T>,
  pub(crate) collector: Option<SparseUpdateCollector>,
}

pub type SparseUpdateCollector =
  Vec<Pin<FrameBox<dyn Future<Output = SparseBufferWritesSource> + Send>>>;

impl<T: Std430 + ShaderSizedValueNodeType> SparseUpdateStorageBuffer<T> {
  pub fn new(
    label: &str,
    init_capacity_item_count: u32,
    max_item_count: u32,
    allocator: &dyn AbstractStorageAllocator,
    gpu: &GPU,
  ) -> Self {
    let buffer = allocator.allocate_readonly(
      make_init_size::<T>(init_capacity_item_count),
      &gpu.device,
      Some(label),
    );

    let buffer = buffer
      .with_direct_resize(gpu)
      .with_default_grow_behavior(max_item_count);

    SparseUpdateStorageBuffer {
      buffer,
      collector: None,
    }
  }
}

impl<T: Std430 + ShaderSizedValueNodeType> SparseUpdateStorageBuffer<T> {
  pub fn get_gpu_buffer(&self) -> AbstractReadonlyStorageBuffer<[T]> {
    self.buffer.gpu().clone()
  }

  // todo, use reactive impl(watch db change)
  pub fn use_max_item_count_by_db_entity<E: EntitySemantic>(&mut self, _cx: &mut QueryGPUHookCx) {
    let size_require =
      database::global_database().access_ecg_dyn(E::entity_id(), |ecg| ecg.entity_capacity());
    self.buffer.check_resize(size_require as u32);
  }

  pub fn use_update(&mut self, cx: &mut QueryGPUHookCx) {
    use_update_impl(cx, &mut self.collector, self.buffer.abstract_gpu());
  }
}

pub type SparseStorageBufferWithHostRaw<T> = CustomGrowBehaviorMaintainer<
  VecWithStorageBuffer<ResizableGPUBuffer<AbstractReadonlyStorageBuffer<[T]>>>,
>;

pub struct SparseUpdateStorageWithHostBuffer<T: Std430> {
  pub buffer: Arc<RwLock<SparseStorageBufferWithHostRaw<T>>>,
  pub(crate) collector: Option<SparseUpdateCollector>,
}

impl<T: Std430 + ShaderSizedValueNodeType> SparseUpdateStorageWithHostBuffer<T> {
  pub fn new(
    label: &str,
    init_capacity_item_count: u32,
    max_item_count: u32,
    allocator: &dyn AbstractStorageAllocator,
    gpu: &GPU,
  ) -> Self {
    let buffer = allocator.allocate_readonly(
      make_init_size::<T>(init_capacity_item_count),
      &gpu.device,
      Some(label),
    );

    let buffer = buffer
      .with_direct_resize(gpu)
      .with_vec_backup(T::zeroed(), false)
      .with_default_grow_behavior(max_item_count);

    SparseUpdateStorageWithHostBuffer {
      buffer: Arc::new(RwLock::new(buffer)),
      collector: None,
    }
  }
}

impl<T: Std430 + ShaderSizedValueNodeType> SparseUpdateStorageWithHostBuffer<T> {
  pub fn get_gpu_buffer(&self) -> AbstractReadonlyStorageBuffer<[T]> {
    self.buffer.read().gpu().clone()
  }

  // todo, use reactive impl(watch db change)
  pub fn use_max_item_count_by_db_entity<E: EntitySemantic>(&mut self, _cx: &mut QueryGPUHookCx) {
    let size_require =
      database::global_database().access_ecg_dyn(E::entity_id(), |ecg| ecg.entity_capacity());
    self.buffer.write().check_resize(size_require as u32);
  }

  // todo, make sure this called in worker
  pub fn write_sparse_updates(&mut self, updates: &SparseBufferWritesSource) {
    let mut buffer = self.buffer.write();
    let host_buffer = &mut buffer.inner.vec;
    let host_buffer: &mut [u8] = cast_slice_mut(host_buffer);

    for (offset, data) in updates.iter_updates() {
      host_buffer[offset..(offset + data.len())].copy_from_slice(data);
    }
  }

  pub fn use_update(&mut self, cx: &mut QueryGPUHookCx) {
    let updates = use_update_impl(cx, &mut self.collector, self.buffer.write().abstract_gpu());
    if let Some(updates) = updates {
      self.write_sparse_updates(&updates);
    }
  }
}

#[inline(never)]
fn use_update_impl(
  cx: &mut QueryGPUHookCx,
  collector: &mut Option<SparseUpdateCollector>,
  buffer: &dyn AbstractBuffer,
) -> Option<Arc<SparseBufferWritesSource>> {
  let (cx, token) = cx.use_plain_state(|| u32::MAX);

  match &mut cx.stage {
    GPUQueryHookStage::Update {
      task_pool, spawner, ..
    } => {
      let collector = collector.take();
      let collector = collector.expect("expect collector exist in task spawn stage");

      if collector.is_empty() {
        *token = u32::MAX;
        return None;
      }

      let spawner = spawner.clone();
      let fut = async move {
        let mut all_writes = join_all(collector).await;

        let r = if all_writes.iter().all(|v| v.is_empty()) {
          SparseBufferWritesSource::default()
        } else if all_writes.len() == 1 {
          all_writes.remove(0)
        } else {
          spawner
            .spawn_task(move || {
              let data_to_write_len = all_writes.iter().map(|v| v.data_to_write.len()).sum();
              let offset_size_len = all_writes.iter().map(|v| v.offset_size.len()).sum();

              let mut target =
                SparseBufferWritesSource::with_capacity(data_to_write_len, offset_size_len);

              all_writes.into_iter().for_each(|w| {
                target.merge(w);
              });

              target
            })
            .await
        };
        Arc::new(r)
      };

      *token = task_pool.install_task(fut);
    }
    GPUQueryHookStage::CreateRender { task, encoder } => {
      if *token == u32::MAX {
        return None;
      }

      // do update in main thread
      let updates = task
        .expect_result_by_id::<Arc<SparseBufferWritesSource>>(*token)
        .clone();
      updates.write_abstract(cx.gpu, encoder, buffer);
      return Some(updates);
    }
  }

  None
}

fn make_init_size<T: Std430>(size: u32) -> u64 {
  ((size as usize) * std::mem::size_of::<T>()) as u64
}
