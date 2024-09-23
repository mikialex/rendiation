use std::ops::DerefMut;

use super::*;

pub struct DeviceTaskSystemBuildCtx<'a> {
  pub compute_cx: &'a mut ShaderComputePipelineBuilder,

  pub(super) self_task_idx: usize,
  pub(super) all_task_group_sources: &'a mut Vec<(TaskGroupExecutorResource, FastHashSet<usize>)>,
  pub(super) tasks_depend_on_self: FastHashMap<usize, TaskGroupDeviceInvocationInstance>,
  pub(super) self_spawner: Arc<RwLock<Option<TaskGroupDeviceInvocationInstance>>>,

  pub state_builder: DynamicTypeBuilder,
}

pub enum TaskGroupDeviceInvocationInstanceMaybeSelf {
  NoneSelfTask(TaskGroupDeviceInvocationInstance),
  SelfTask(Arc<RwLock<Option<TaskGroupDeviceInvocationInstance>>>),
}

impl TaskGroupDeviceInvocationInstanceMaybeSelf {
  #[must_use]
  pub fn poll_task<T: ShaderSizedValueNodeType>(
    &self,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(Node<T>) + Copy,
  ) -> Node<bool> {
    self.poll_task_dyn(task_id, |x| unsafe {
      argument_read_back(x.cast_type::<ShaderStoragePtr<T>>().load())
    })
  }

  #[must_use]
  pub fn poll_task_dyn(
    &self,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(StorageNode<AnyType>) + Copy,
  ) -> Node<bool> {
    match self {
      TaskGroupDeviceInvocationInstanceMaybeSelf::NoneSelfTask(inner) => {
        inner.poll_task_dyn(task_id, argument_read_back)
      }
      TaskGroupDeviceInvocationInstanceMaybeSelf::SelfTask(arc) => {
        let inner = arc.read_recursive();
        inner
          .as_ref()
          .unwrap()
          .poll_task_dyn(task_id, argument_read_back)
      }
    }
  }

  #[must_use]
  pub fn spawn_new_task<T: ShaderSizedValueNodeType>(
    &self,
    payload: Node<T>,
    parent_ref: TaskParentRef,
  ) -> Option<TaskFutureInvocationRightValue> {
    self.spawn_new_task_dyn(payload.cast_untyped_node(), parent_ref, &T::sized_ty())
  }

  #[must_use]
  pub fn spawn_new_task_dyn(
    &self,
    payload: Node<AnyType>,
    parent_ref: TaskParentRef,
    ty: &ShaderSizedValueType,
  ) -> Option<TaskFutureInvocationRightValue> {
    match self {
      TaskGroupDeviceInvocationInstanceMaybeSelf::NoneSelfTask(inner) => {
        inner.spawn_new_task_dyn(payload, parent_ref, ty)
      }
      TaskGroupDeviceInvocationInstanceMaybeSelf::SelfTask(arc) => {
        let inner = arc.read_recursive();
        inner
          .as_ref()
          .unwrap()
          .spawn_new_task_dyn(payload, parent_ref, ty)
      }
    }
  }
}

impl<'a> DeviceTaskSystemBuildCtx<'a> {
  // todo, handle self task spawner
  pub fn get_or_create_task_group_instance(
    &mut self,
    task_type: usize,
  ) -> TaskGroupDeviceInvocationInstanceMaybeSelf {
    if task_type == self.self_task_idx {
      return TaskGroupDeviceInvocationInstanceMaybeSelf::SelfTask(self.self_spawner.clone());
    }

    let instance = self
      .tasks_depend_on_self
      .entry(task_type)
      .or_insert_with(|| {
        let source = &mut self.all_task_group_sources[task_type];
        source.1.insert(self.self_task_idx);
        source.0.build_shader_for_spawner(self.compute_cx)
      })
      .clone();

    TaskGroupDeviceInvocationInstanceMaybeSelf::NoneSelfTask(instance)
  }
}

pub struct DeviceTaskSystemBindCtx<'a> {
  pub binder: &'a mut BindingBuilder,
  pub self_task_idx: usize,

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
    if task_type == self.self_task_idx {
      return;
    }
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
  pub(super) self_task_type_id: u32,
  pub(super) self_task: TaskPoolInvocationInstance,
  pub invocation_registry: AnyMap,
}

impl<'a> DeviceTaskSystemPollCtx<'a> {
  pub fn generate_self_as_parent(&self) -> TaskParentRef {
    TaskParentRef {
      parent_task_index: self.self_task_idx,
      parent_task_type_id: val(self.self_task_type_id),
    }
  }
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

  pub fn access_self_payload_untyped(&mut self) -> StorageNode<AnyType> {
    let current = self.self_task_idx;
    self.self_task.rw_payload_dyn(current)
  }
}
