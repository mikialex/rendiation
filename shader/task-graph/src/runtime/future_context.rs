use super::*;

pub struct DeviceTaskSystemBuildCtx<'a> {
  pub compute_cx: &'a mut ShaderComputePipelineBuilder,
  pub state_builder: DynamicTypeBuilder,
}

pub struct DeviceTaskSystemPollCtx<'a> {
  pub(super) compute_cx: &'a mut ShaderComputePipelineBuilder,
  pub(super) all_task_group_sources: Vec<&'a TaskGroupExecutorResource>,
  pub(super) self_task_idx: Node<u32>,
  pub(super) self_task: TaskPoolInvocationInstance,
  pub(super) tasks_depend_on_self: FastHashMap<usize, TaskGroupDeviceInvocationInstance>,
  // the rust hashmap is not ordered
  pub(super) tasks_depend_on_self_bind_order: Vec<usize>,
  pub registry: &'a mut FastHashMap<TypeId, Box<dyn Any>>,
}

impl<'a> DeviceTaskSystemPollCtx<'a> {
  // todo, handle self task spawner
  fn get_or_create_task_group_instance(
    &mut self,
    task_type: usize,
  ) -> &mut TaskGroupDeviceInvocationInstance {
    self
      .tasks_depend_on_self
      .entry(task_type)
      .or_insert_with(|| {
        let source = &self.all_task_group_sources[task_type];
        self.tasks_depend_on_self_bind_order.push(task_type);
        source.build_shader_for_spawner(self.compute_cx)
      })
  }

  pub fn access_self_payload<T: ShaderSizedValueNodeType>(&mut self) -> StorageNode<T> {
    let current = self.self_task_idx;
    self.self_task.rw_payload(current)
  }

  pub fn spawn_task<T: ShaderSizedValueNodeType>(
    &mut self,
    task_type: usize,
    argument: Node<T>,
  ) -> Option<TaskFutureInvocationRightValue> {
    let task_group = self.get_or_create_task_group_instance(task_type);
    TaskFutureInvocationRightValue {
      task_handle: task_group.spawn_new_task(argument)?,
    }
    .into()
  }

  pub fn poll_task<T: ShaderSizedValueNodeType>(
    &mut self,
    task_type: usize,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
  ) -> Node<bool> {
    let task_group = self.get_or_create_task_group_instance(task_type);
    let finished = task_group.poll_task_is_finished(task_id);
    if_by(finished, || {
      argument_read_back(task_group.read_back_payload(task_id));
      task_group.cleanup_finished_task_state_and_payload(task_id)
    });
    finished
  }
}
