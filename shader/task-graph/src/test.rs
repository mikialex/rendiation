#[pollster::test]
async fn test_task_graph() {
  use crate::*;

  let (gpu, _) = GPU::new(Default::default()).await.unwrap();

  let mut graph = DeviceTaskGraphExecutor::new(12, 1);

  let mut cx = DeviceParallelComputeCtx::new(&gpu);

  let test_task = graph.define_task::<u32, _>(BaseDeviceFuture::default(), &mut cx);

  let test_task2 = graph.define_task::<u32, _>(
    BaseDeviceFuture::default()
      .then(
        |_: (), then, _| then.spawner.spawn_new_task(val(0_u32)).unwrap(),
        TaskFuture::<u32>::new(test_task as usize),
      )
      .map(|_, _| {}),
    &mut cx,
  );

  let work_size = 3;

  println!("{:?}", graph.debug_execution(&mut cx).await);

  graph.dispatch_allocate_init_task(&mut cx, work_size, test_task2, |_| val(0_u32));

  cx.submit_recorded_work_and_continue();

  // let round = graph.compute_conservative_dispatch_round_count();
  // assert!(round >= 3);

  println!("{:?}", graph.debug_execution(&mut cx).await);

  let info = graph.read_back_execution_states(&mut cx).await;
  assert_eq!(info.remain_task_counts[test_task as usize], 0);
  assert_eq!(info.remain_task_counts[test_task2 as usize], work_size);

  graph.execute(&mut cx, 1);

  let info = graph.read_back_execution_states(&mut cx).await;
  assert_eq!(info.remain_task_counts[test_task as usize], work_size);
  assert_eq!(info.remain_task_counts[test_task2 as usize], work_size);

  graph.execute(&mut cx, 1);

  let info = graph.read_back_execution_states(&mut cx).await;

  // println!("{:?}", graph.debug_execution(&mut cx).await);

  assert_eq!(info.remain_task_counts[test_task as usize], 0);
  assert_eq!(info.remain_task_counts[test_task2 as usize], 0); // 62?
}
