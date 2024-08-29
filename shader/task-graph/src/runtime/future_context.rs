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
  pub registry: &'a mut AnyMap,
  pub invocation_registry: AnyMap,
}

#[derive(Default)]
pub struct AnyMap {
  map: FastHashMap<TypeId, Box<dyn Any>>,
}

impl AnyMap {
  pub fn register<T: Any>(&mut self, value: T) {
    self.map.insert(TypeId::of::<T>(), Box::new(value));
  }
  pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
    self
      .map
      .get_mut(&TypeId::of::<T>())
      .and_then(|x| x.downcast_mut())
  }
}

impl<'a> DeviceTaskSystemPollCtx<'a> {
  // todo, handle self task spawner
  pub fn get_or_create_task_group_instance(
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
    self.spawn_task_dyn(task_type, argument.cast_untyped_node(), &T::sized_ty())
  }

  pub fn spawn_task_dyn(
    &mut self,
    task_type: usize,
    argument: Node<AnyType>,
    argument_ty: &ShaderSizedValueType,
  ) -> Option<TaskFutureInvocationRightValue> {
    let task_group = self.get_or_create_task_group_instance(task_type);
    TaskFutureInvocationRightValue {
      task_handle: task_group.spawn_new_task_dyn(argument, argument_ty)?,
    }
    .into()
  }

  pub fn poll_task<T: ShaderSizedValueNodeType>(
    &mut self,
    task_type: usize,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
  ) -> Node<bool> {
    self.poll_task_dyn(task_type, task_id, |x| unsafe {
      argument_read_back(x.cast_type::<ShaderStoragePtr<T>>().load())
    })
  }

  pub fn poll_task_dyn(
    &mut self,
    task_type: usize,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(StorageNode<AnyType>) + Copy,
  ) -> Node<bool> {
    let task_group = self.get_or_create_task_group_instance(task_type);
    let finished = task_group.poll_task_is_finished(task_id);
    if_by(finished, || {
      argument_read_back(task_group.rw_payload_dyn(task_id));
      task_group.cleanup_finished_task_state_and_payload(task_id)
    });
    finished
  }
}
