use std::ops::DerefMut;

use super::*;

pub struct DeviceTaskSystemBuildCtx<'a> {
  pub compute_cx: &'a mut ShaderComputePipelineBuilder,

  pub(super) self_task_idx: usize,
  pub(super) task_group_shared_info: &'a mut Vec<(
    TaskGroupDeviceInvocationInstanceLateResolved,
    FastHashSet<usize>,
  )>,
  pub(super) tasks_depend_on_self:
    FastHashMap<usize, TaskGroupDeviceInvocationInstanceLateResolved>,

  pub state_builder: DynamicTypeBuilder,
}

impl<'a> DeviceTaskSystemBuildCtx<'a> {
  /// just inner method short cut
  pub fn make_state<T: ShaderAbstractRightValue>(&mut self) -> T::AbstractLeftValue {
    self
      .state_builder
      .create_or_reconstruct_any_left_value_by_right::<T>()
  }
}

#[derive(Clone, Default)]
pub struct TaskGroupDeviceInvocationInstanceLateResolved {
  inner: Arc<RwLock<Option<TaskGroupDeviceInvocationInstance>>>,
}

impl TaskGroupDeviceInvocationInstanceLateResolved {
  pub fn resolve(&self, instance: TaskGroupDeviceInvocationInstance) {
    self.inner.write().replace(instance);
  }

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
    let inner = self.inner.read_recursive();
    inner
      .as_ref()
      .expect("source not resolved")
      .poll_task_dyn(task_id, argument_read_back)
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
    let inner = self.inner.read_recursive();
    inner
      .as_ref()
      .expect("source not resolved")
      .spawn_new_task_dyn(payload, parent_ref, ty)
  }
}

impl<'a> DeviceTaskSystemBuildCtx<'a> {
  pub fn get_or_create_task_group_instance(
    &mut self,
    task_type: usize,
  ) -> TaskGroupDeviceInvocationInstanceLateResolved {
    self
      .tasks_depend_on_self
      .entry(task_type)
      .or_insert_with(|| {
        let source = &mut self.task_group_shared_info[task_type];
        source.1.insert(self.self_task_idx);
        source.0.clone()
      })
      .clone()
  }
}

pub struct DeviceTaskSystemBindCtx<'a> {
  pub binder: &'a mut BindingBuilder,
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
  pub fn get<T: Any>(&self) -> Option<&T> {
    self
      .map
      .get(&TypeId::of::<T>())
      .and_then(|x| x.downcast_ref())
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
