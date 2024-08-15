#[pollster::test]
async fn test_task_graph() {
  use crate::*;

  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut graph = DeviceTaskGraphExecutor::new(128, 1);

  let mut encoder = gpu.create_encoder();

  let mut pass = encoder.begin_compute_pass();

  let test_task = graph.define_task::<u32, _>(BaseDeviceFuture::default(), &gpu.device, &mut pass);

  let test_task2 = graph.define_task::<u32, _>(BaseDeviceFuture::default(), &gpu.device, &mut pass);

  graph.dispatch_allocate_init_task(&gpu.device, &mut pass, 64, test_task, |_| val(0_u32));
  drop(pass);

  gpu.submit_encoder(encoder);

  let mut cx = DeviceParallelComputeCtx::new(&gpu);
  let info = graph.read_back_execution_states(&mut cx).await;
  assert_eq!(info.remain_task_counts[test_task as usize], 64);
  assert_eq!(info.remain_task_counts[test_task2 as usize], 0);

  let round = graph.compute_conservative_dispatch_round_count();
  assert_eq!(round, 2);
  graph.execute(&mut cx, round);

  let info = graph.read_back_execution_states(&mut cx).await;
  dbg!(info);
}
