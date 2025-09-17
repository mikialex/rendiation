use std::{future::Future, pin::Pin};

use futures::future::join_all;

use crate::*;

type SparseStorageBufferRaw<T> =
  CustomGrowBehaviorMaintainer<ResizableGPUBuffer<AbstractReadonlyStorageBuffer<[T]>>>;

pub struct SparseUpdateStorageBuffer<T> {
  buffer: SparseStorageBufferRaw<T>,
  collector: Option<SparseUpdateCollector>,
}

impl<T: Std430 + ShaderSizedValueNodeType> SparseUpdateStorageBuffer<T> {
  pub fn get_gpu_buffer(&self) -> AbstractReadonlyStorageBuffer<[T]> {
    self.buffer.gpu().clone()
  }

  pub fn sync_max_item_count(&mut self, item_count: u32) {
    self.buffer.check_resize(item_count);
  }

  pub fn use_update(&mut self, cx: &mut QueryGPUHookCx) {
    use_update_impl(cx, &mut self.collector, self.buffer.abstract_gpu());
  }
}

#[inline(never)]
fn use_update_impl(
  cx: &mut QueryGPUHookCx,
  collector: &mut Option<SparseUpdateCollector>,
  buffer: &dyn AbstractBuffer,
) {
  let (cx, token) = cx.use_plain_state(|| u32::MAX);

  match &mut cx.stage {
    GPUQueryHookStage::Update {
      task_pool, spawner, ..
    } => {
      let collector = collector.take();
      let collector = collector.expect("expect collector exist in task spawn stage");

      let spawner = spawner.clone();
      let fut = async move {
        let mut all_writes = join_all(collector.waiter).await;

        let r = if all_writes.iter().all(|v| v.is_empty()) {
          SparseBufferWritesSource::default()
        } else if all_writes.len() == 1 {
          all_writes.remove(0)
        } else {
          spawner
            .spawn_task(move || {
              // todo, remove this possible allocation using small vec?
              let (data_to_write, offset_size): (Vec<_>, Vec<_>) = all_writes
                .into_iter()
                .map(|v| (v.data_to_write, v.offset_size))
                .unzip();

              SparseBufferWritesSource {
                data_to_write: concat_iter_of_vec(
                  data_to_write.iter().map(|v| v.len()).sum(),
                  data_to_write.into_iter(),
                ),
                offset_size: concat_iter_of_vec(
                  offset_size.iter().map(|v| v.len()).sum(),
                  offset_size.into_iter(),
                ),
              }
            })
            .await
        };
        Arc::new(r)
      };

      *token = task_pool.install_task(fut);
    }
    GPUQueryHookStage::CreateRender { task } => {
      // do update in main thread
      let updates = task.expect_result_by_id::<Arc<SparseBufferWritesSource>>(*token);

      // todo, this may failed if we support texture as storage buffer
      let target_buffer = buffer.get_gpu_buffer_view().unwrap();
      let mut encoder = cx.gpu.create_encoder(); // todo, reuse encoder and pass
      encoder.compute_pass_scoped(|mut pass| {
        updates.write(&cx.gpu.device, &mut pass, target_buffer);
      });
      cx.gpu.queue.submit_encoder(encoder);
    }
    _ => {}
  }
}

struct SparseUpdateCollector {
  waiter: Vec<Pin<Box<dyn Future<Output = SparseBufferWritesSource> + Send>>>,
}

fn concat_iter_of_vec<'a, T: 'a>(size_all: usize, iter: impl Iterator<Item = Vec<T>>) -> Vec<T> {
  // we don't use iter flat_map then collect, because flat map can not avoid resize
  let mut target = Vec::with_capacity(size_all);
  for v in iter {
    target.extend(v);
  }
  target
}
