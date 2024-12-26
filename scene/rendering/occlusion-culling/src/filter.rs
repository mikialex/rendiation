use crate::*;

pub fn filter_last_frame_visible_object(
  last_frame: &StorageBufferDataView<[Bool]>,
  batch: &DeviceSceneModelRenderBatch,
) -> DeviceSceneModelRenderBatch {
  #[derive(Clone)]
  struct Filter {
    last_frame: StorageBufferDataView<[Bool]>,
    input: Box<dyn DeviceParallelComputeIO<u32>>,
  }

  impl DeviceParallelCompute<Node<u32>> for Filter {
    fn execute_and_expose(
      &self,
      cx: &mut DeviceParallelComputeCtx,
    ) -> Box<dyn DeviceInvocationComponent<Node<u32>>> {
      todo!()
    }

    fn result_size(&self) -> u32 {
      todo!()
    }
  }

  impl DeviceParallelComputeIO<u32> for Filter {}

  let sub_batches = batch
    .sub_batches
    .iter()
    .map(|sub_batch| {
      let scene_models = Box::new(Filter {
        last_frame: last_frame.clone(),
        input: sub_batch.scene_models.clone(),
      });

      DeviceSceneModelRenderSubBatch {
        scene_models,
        impl_select_id: sub_batch.impl_select_id,
      }
    })
    .collect();

  DeviceSceneModelRenderBatch { sub_batches }
}
