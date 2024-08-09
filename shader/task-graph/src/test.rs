use crate::*;

#[pollster::test]
async fn test_task_graph() {
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut graph = DeviceTaskGraphExecutor::new(128);

  let mut encoder = gpu.create_encoder();
  let mut pass = encoder.begin_compute_pass();
  graph.define_task(BaseDeviceFuture::default(), || {}, &gpu.device, &mut pass);
}
