use super::*;

pub struct DeviceTaskSystemBuildCtx<'a> {
  pub compute_cx: &'a mut ShaderComputePipelineBuilder,

  pub(super) all_task_group_sources: Vec<&'a TaskGroupExecutorResource>,
  pub(super) tasks_depend_on_self: FastHashMap<usize, TaskGroupDeviceInvocationInstance>,
  // the rust hashmap is not ordered
  pub(super) tasks_depend_on_self_bind_order: Vec<usize>,

  pub state_builder: DynamicTypeBuilder,
}

impl<'a> DeviceTaskSystemBuildCtx<'a> {
  // todo, handle self task spawner
  pub fn get_or_create_task_group_instance(
    &mut self,
    task_type: usize,
  ) -> TaskGroupDeviceInvocationInstance {
    self
      .tasks_depend_on_self
      .entry(task_type)
      .or_insert_with(|| {
        let source = &self.all_task_group_sources[task_type];
        self.tasks_depend_on_self_bind_order.push(task_type);
        source.build_shader_for_spawner(self.compute_cx)
      })
      .clone()
  }
}

pub struct DeviceTaskSystemPollCtx<'a> {
  pub compute_cx: &'a mut ShaderComputePipelineBuilder,
  pub(super) self_task_idx: Node<u32>,
  pub(super) self_task: TaskPoolInvocationInstance,
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
  pub fn access_self_payload<T: ShaderSizedValueNodeType>(&mut self) -> StorageNode<T> {
    let current = self.self_task_idx;
    self.self_task.rw_payload(current)
  }
}
