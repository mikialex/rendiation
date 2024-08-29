use std::ops::DerefMut;

use super::*;

pub struct DeviceTaskSystemBuildCtx<'a> {
  pub compute_cx: &'a mut ShaderComputePipelineBuilder,

  pub(super) all_task_group_sources: Vec<&'a TaskGroupExecutorResource>,
  pub(super) tasks_depend_on_self: FastHashMap<usize, TaskGroupDeviceInvocationInstance>,

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
        source.build_shader_for_spawner(self.compute_cx)
      })
      .clone()
  }
}

pub struct DeviceTaskSystemBindCtx<'a> {
  pub binder: &'a mut BindingBuilder,

  pub(super) all_task_group_sources: Vec<&'a TaskGroupExecutorResource>,
  pub bound_task_group_instance: FastHashSet<usize>,
}

impl<'a> DerefMut for DeviceTaskSystemBindCtx<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.binder
  }
}

impl<'a> std::ops::Deref for DeviceTaskSystemBindCtx<'a> {
  type Target = BindingBuilder;

  fn deref(&self) -> &Self::Target {
    self.binder
  }
}

impl<'a> DeviceTaskSystemBindCtx<'a> {
  pub fn bind_task_group_instance(&mut self, task_type: usize) {
    self
      .bound_task_group_instance
      .get_or_insert_with(&task_type, |_| {
        self.all_task_group_sources[task_type].bind_for_spawner(self.binder);
        task_type
      });
  }
}

pub struct DeviceTaskSystemPollCtx<'a> {
  pub compute_cx: &'a mut ShaderComputePipelineBuilder,
  pub(super) self_task_idx: Node<u32>,
  pub(super) self_task: TaskPoolInvocationInstance,
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
