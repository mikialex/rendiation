#[pollster::test]
async fn test_simple_map() {
  use crate::*;

  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut graph = DeviceTaskGraphBuildSource::default();

  let test_task = graph.define_task::<u32, _>(BaseShaderFuture::default().map(|_: (), _| {}));
  let test_task2 = graph.define_task::<u32, _>(BaseShaderFuture::default());

  let mut cx = DeviceParallelComputeCtx::new(&gpu);
  let mut graph = graph.build(12, 1, &mut cx);

  let work_size = 3;
  let work_size2 = 4;

  graph.dispatch_allocate_init_task(&mut cx, work_size, test_task, |_| val(0_u32));
  graph.dispatch_allocate_init_task(&mut cx, work_size2, test_task2, |_| val(0_u32));
  cx.submit_recorded_work_and_continue();

  let info = graph.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], work_size);
  assert_eq!(info.wake_counts[test_task2 as usize], work_size2);

  graph.execute(&mut cx, 1);

  let info = graph.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], 0);
  assert_eq!(info.wake_counts[test_task2 as usize], 0);
}

#[pollster::test]
async fn test_task_graph_then_task_spawn() {
  use crate::*;

  // #[repr(C)]
  // #[derive(Clone, Copy, Debug, Zeroable, Pod)]
  // struct State {
  //   is_finished: u32,
  //   payload: u32,
  //   states_0: u32,
  //   // states_1: u32,
  //   // states_2: u32,
  //   parent_task_type_id: u32,
  //   parent_task_index: u32,
  // }

  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut graph = DeviceTaskGraphBuildSource::default();

  let test_task = graph.define_task::<u32, _>(BaseShaderFuture::default());

  let test_task2 = graph.define_task::<u32, _>(
    BaseShaderFuture::default()
      .then(
        |_: (), then, cx| {
          then
            .spawner
            .spawn_new_task(val(0_u32), cx.generate_self_as_parent())
            .unwrap()
        },
        TaskFuture::<u32>::new(test_task as usize),
      )
      .map(|_, _| {}),
  );

  let mut cx = DeviceParallelComputeCtx::new(&gpu);
  let mut graph = graph.build(4, 1, &mut cx);

  let work_size = 3;

  graph.dispatch_allocate_init_task(&mut cx, work_size, test_task2, |_| val(0_u32));
  cx.submit_recorded_work_and_continue();

  // let debug_info = graph.debug_execution(&mut cx).await;
  // println!("{:?}", debug_info);

  // dbg!(cast_slice::<u8, State>(&debug_info.info[1].task_states));

  let info = graph.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], 0);
  assert_eq!(info.wake_counts[test_task2 as usize], work_size);

  graph.execute(&mut cx, 1);

  // let debug_info = graph.debug_execution(&mut cx).await;
  // println!("{:?}", debug_info);
  // dbg!(cast_slice::<u8, State>(
  //   &debug_info.info[test_task as usize].task_states
  // ));

  let info = graph.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], work_size);
  assert_eq!(info.sleep_or_finished_counts[test_task as usize], 0);
  assert_eq!(info.wake_counts[test_task2 as usize], 0);
  assert_eq!(
    info.sleep_or_finished_counts[test_task2 as usize],
    work_size
  );

  graph.execute(&mut cx, 1);

  let info = graph.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], 0);
  assert_eq!(info.sleep_or_finished_counts[test_task as usize], 0);
  assert_eq!(info.wake_counts[test_task2 as usize], 0);
  assert_eq!(info.sleep_or_finished_counts[test_task2 as usize], 0);
}

#[pollster::test]
async fn test_task_graph_then_task_self_spawn_recursive() {
  use crate::*;

  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut graph = DeviceTaskGraphBuildSource::default();

  let test_task = graph.next_task_idx();
  let test_task = graph.define_task::<u32, _>(
    BaseShaderFuture::default()
      .then(
        |_: (), then, cx| {
          then
            .spawner
            .spawn_new_task(val(0_u32), cx.generate_self_as_parent())
            .unwrap()
        },
        TaskFuture::<u32>::new(test_task as usize),
      )
      .map(|_, _| {}),
  );

  let mut cx = DeviceParallelComputeCtx::new(&gpu);
  let mut graph = graph.build(4, 3, &mut cx);

  let work_size = 3;

  graph.dispatch_allocate_init_task(&mut cx, work_size, test_task, |_| val(0_u32));
  cx.submit_recorded_work_and_continue();

  let info = graph.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], work_size);

  graph.execute(&mut cx, 1);

  let info = graph.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], work_size);
  assert_eq!(info.sleep_or_finished_counts[test_task as usize], work_size);

  graph.execute(&mut cx, 1);

  let info = graph.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], work_size);
  assert_eq!(
    info.sleep_or_finished_counts[test_task as usize],
    work_size * 2
  );

  graph.execute(&mut cx, 1);

  let info = graph.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], work_size);
  assert_eq!(
    info.sleep_or_finished_counts[test_task as usize],
    work_size * 3
  );
}
