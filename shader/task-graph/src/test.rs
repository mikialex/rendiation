#[pollster::test]
async fn test_simple_map() {
  use crate::*;

  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut graph = DeviceTaskGraphBuildSource::default();

  let test_task = graph.define_task::<u32, _>(BaseShaderFuture::default().map(|_: (), _| {}), 2);
  let test_task2 = graph.define_task::<u32, _>(BaseShaderFuture::default(), 2);
  graph.capacity = 12;

  let mut encoder = gpu.create_encoder();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder);
  let mut graph_exe = graph.build(&mut cx, false);

  let work_size = 3;
  let work_size2 = 4;

  graph_exe.dispatch_allocate_init_task_by_fn(&mut cx, work_size, test_task, |_| val(0_u32));
  graph_exe.dispatch_allocate_init_task_by_fn(&mut cx, work_size2, test_task2, |_| val(0_u32));
  cx.submit_recorded_work_and_continue();

  let info = graph_exe.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], work_size);
  assert_eq!(info.wake_counts[test_task2 as usize], work_size2);

  graph_exe.execute(&mut cx, 1, &graph);

  let info = graph_exe.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], 0);
  assert_eq!(info.wake_counts[test_task2 as usize], 0);
}

#[pollster::test]
async fn test_task_graph_then_task_spawn() {
  use crate::*;

  #[repr(C)]
  #[derive(Clone, Copy, Debug, Zeroable, Pod)]
  struct Task1State {
    is_finished: u32,
    payload: u32,
    states_0: u32,
    states_1: u32,
    parent_task_type_id: u32,
    parent_task_index: u32,
  }

  #[repr(C)]
  #[derive(Clone, Copy, Debug, Zeroable, Pod)]
  struct Task2State {
    is_finished: u32,
    payload: u32,
    states_0: u32,
    states_1: u32,
    states_2: u32,
    parent_task_type_id: u32,
    parent_task_index: u32,
  }

  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut graph = DeviceTaskGraphBuildSource::default();
  graph.capacity = 4;

  let test_task = graph.define_task::<u32, _>(BaseShaderFuture::default(), 2);

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
    2,
  );

  let mut encoder = gpu.create_encoder();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder);
  let mut graph_exe = graph.build(&mut cx, false);

  let work_size = 3;

  let enable_debug = false; // enable this may output too much debug info
  let debug_detail_states = |debug_info: &TaskGraphExecutionDebugInfo| {
    if enable_debug {
      dbg!(cast_slice::<u8, Task1State>(
        &debug_info.info[test_task as usize].task_states
      ));
      dbg!(cast_slice::<u8, Task2State>(
        &debug_info.info[test_task2 as usize].task_states
      ));
    }
  };

  let mut test_round = async || {
    println!("test round:");
    graph_exe.dispatch_allocate_init_task_by_fn(&mut cx, work_size, test_task2, |_| val(0_u32));
    cx.submit_recorded_work_and_continue();

    let debug_info = graph_exe.debug_execution(&mut cx).await;
    println!("{:?}", debug_info);
    debug_detail_states(&debug_info);

    let info = graph_exe.read_back_execution_states(&mut cx).await;
    assert_eq!(info.wake_counts[test_task as usize], 0);
    assert_eq!(info.wake_counts[test_task2 as usize], work_size);

    graph_exe.execute(&mut cx, 1, &graph);

    let debug_info = graph_exe.debug_execution(&mut cx).await;
    println!("{:?}", debug_info);
    debug_detail_states(&debug_info);

    let info = graph_exe.read_back_execution_states(&mut cx).await;
    assert_eq!(info.wake_counts[test_task as usize], work_size);
    assert_eq!(info.sleep_or_finished_counts[test_task as usize], 0);
    assert_eq!(info.wake_counts[test_task2 as usize], 0);
    assert_eq!(
      info.sleep_or_finished_counts[test_task2 as usize],
      work_size
    );

    graph_exe.execute(&mut cx, 1, &graph);

    let debug_info = graph_exe.debug_execution(&mut cx).await;
    println!("{:?}", debug_info);
    debug_detail_states(&debug_info);

    let info = graph_exe.read_back_execution_states(&mut cx).await;
    assert_eq!(info.wake_counts[test_task as usize], 0);
    assert_eq!(info.sleep_or_finished_counts[test_task as usize], 0);
    assert_eq!(info.wake_counts[test_task2 as usize], 0);
    assert_eq!(info.sleep_or_finished_counts[test_task2 as usize], 0);
  };

  test_round().await;
  test_round().await;
}

#[pollster::test]
async fn test_task_graph_then_task_self_spawn_recursive() {
  use crate::*;

  let (gpu, _) = GPU::new(Default::default()).await.unwrap();
  let mut graph = DeviceTaskGraphBuildSource::default();
  graph.capacity = 4;

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
    4,
  );

  let mut encoder = gpu.create_encoder();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder);
  let mut graph_exe = graph.build(&mut cx, false);

  let work_size = 3;

  graph_exe.dispatch_allocate_init_task_by_fn(&mut cx, work_size, test_task, |_| val(0_u32));
  cx.submit_recorded_work_and_continue();

  let info = graph_exe.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], work_size);

  graph_exe.execute(&mut cx, 1, &graph);

  let info = graph_exe.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], work_size);
  assert_eq!(info.sleep_or_finished_counts[test_task as usize], work_size);

  graph_exe.execute(&mut cx, 1, &graph);

  let info = graph_exe.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], work_size);
  assert_eq!(
    info.sleep_or_finished_counts[test_task as usize],
    work_size * 2
  );

  graph_exe.execute(&mut cx, 1, &graph);

  let info = graph_exe.read_back_execution_states(&mut cx).await;
  assert_eq!(info.wake_counts[test_task as usize], work_size);
  assert_eq!(
    info.sleep_or_finished_counts[test_task as usize],
    work_size * 3
  );
}
