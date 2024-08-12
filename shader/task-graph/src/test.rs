#[pollster::test]
async fn test_task_graph() {
  use crate::*;

  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut graph = DeviceTaskGraphExecutor::new(128);

  let mut encoder = gpu.create_encoder();
  {
    let mut pass = encoder.begin_compute_pass();
    let test_task =
      graph.define_task::<u32, _>(BaseDeviceFuture::default(), || {}, &gpu.device, &mut pass);

    graph.dispatch_allocate_init_task(&gpu.device, &mut pass, 64, test_task, |_| val(0_u32));
  }

  gpu.submit_encoder(encoder);
}
