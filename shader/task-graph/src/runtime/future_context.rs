use std::ops::DerefMut;

use anymap::AnyMap;

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

impl DeviceTaskSystemBuildCtx<'_> {
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
    self.poll_task_dyn(task_id, |x| {
      let argument = T::create_view_from_raw_ptr(x).load();
      argument_read_back(argument)
    })
  }

  #[must_use]
  pub fn poll_task_dyn(
    &self,
    task_id: Node<u32>,
    argument_read_back: impl FnOnce(BoxedShaderPtr) + Copy,
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
    self.spawn_new_task_dyn(payload.handle(), parent_ref, &T::sized_ty())
  }

  #[must_use]
  pub fn spawn_new_task_dyn(
    &self,
    payload: ShaderNodeRawHandle,
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

impl DeviceTaskSystemBuildCtx<'_> {
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

impl DerefMut for DeviceTaskSystemBindCtx<'_> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.binder
  }
}

impl std::ops::Deref for DeviceTaskSystemBindCtx<'_> {
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

impl DeviceTaskSystemPollCtx<'_> {
  pub fn is_fallback_task(&self) -> Node<bool> {
    self.self_task_idx.equals(val(0))
  }
  pub fn generate_self_as_parent(&self) -> TaskParentRef {
    TaskParentRef {
      parent_task_index: self.self_task_idx,
      parent_task_type_id: val(self.self_task_type_id),
    }
  }
}

impl DeviceTaskSystemPollCtx<'_> {
  pub fn access_self_payload<T: ShaderSizedValueNodeType>(&mut self) -> ShaderPtrOf<T> {
    let current = self.self_task_idx;
    self.self_task.rw_payload::<T>(current)
  }

  pub fn access_self_payload_untyped(&mut self) -> BoxedShaderPtr {
    let current = self.self_task_idx;
    self.self_task.rw_payload_dyn(current)
  }
}
