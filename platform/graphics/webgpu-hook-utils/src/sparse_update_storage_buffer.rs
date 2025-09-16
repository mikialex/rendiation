use std::{future::Future, pin::Pin};

use futures::{future::join_all, FutureExt};

use crate::*;

pub struct SparseUpdateStorageBuffer<T> {
  buffer: SparseStorageBufferRaw<T>,
  collector: Option<SparseUpdateCollector>,
}

impl<T> SparseUpdateStorageBuffer<T> {
  pub fn get_gpu_buffer(&self) -> AbstractReadonlyStorageBuffer<[T]> {
    todo!()
  }

  pub fn use_update(&mut self, cx: &mut QueryGPUHookCx) {
    let (cx, token) = cx.use_plain_state(|| u32::MAX);

    match &mut cx.stage {
      GPUQueryHookStage::Update {
        task_pool, spawner, ..
      } => {
        let collector = self.collector.take();
        let collector = collector.expect("expect collector exist in task spawn stage");
        *token = task_pool.install_task(collector.combine(spawner));
      }
      GPUQueryHookStage::CreateRender { task } => {
        // do update in main thread
        task.expect_result_by_id::<Arc<SparseBufferWritesSource>>(*token);
        todo!()
      }
      _ => {}
    }
  }
}

struct SparseUpdateCollector {
  waiter: Vec<Pin<Box<dyn Future<Output = SparseBufferWritesSource> + Send>>>,
}

impl SparseUpdateCollector {
  #[inline(never)]
  pub fn combine(
    self,
    spawner: &TaskSpawner,
  ) -> impl Future<Output = Arc<SparseBufferWritesSource>> + Send + 'static {
    join_all(self.waiter).map(|all_changes| Arc::new(SparseBufferWritesSource::default()))
  }
}

type SparseStorageBufferRaw<T> =
  CustomGrowBehaviorMaintainer<ResizableGPUBuffer<AbstractReadonlyStorageBuffer<[T]>>>;

// impl<T: Std430> LinearStorageBase for SparseUpdateStorageBuffer<T> {
//   type Item = T;

//   fn max_size(&self) -> u32 {
//     self.buffer.max_size()
//   }
// }

// impl<T: Std430> LinearStorageDirectAccess for SparseUpdateStorageBuffer<T> {
//   fn remove(&mut self, idx: u32) -> Option<()> {
//     todo!()
//   }

//   fn set_value(&mut self, idx: u32, v: Self::Item) -> Option<()> {
//     todo!()
//   }

//   unsafe fn set_value_sub_bytes(
//     &mut self,
//     idx: u32,
//     field_byte_offset: usize,
//     v: &[u8],
//   ) -> Option<()> {
//     todo!()
//   }
//   //
// }
