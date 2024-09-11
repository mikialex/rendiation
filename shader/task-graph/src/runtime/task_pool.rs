use crate::*;

#[derive(Clone)]
pub struct TaskPool {
  /// struct Task {
  ///   is_finished_ bool,
  ///   payload: P,
  ///   state: S,
  /// }
  pub(crate) tasks: GPUBufferResourceView,
  state_desc: DynamicTypeMetaInfo,
  task_ty_desc: ShaderStructMetaInfo,
}

impl TaskPool {
  pub fn create_with_size(
    size: usize,
    state_desc: DynamicTypeMetaInfo,
    task_ty_desc: ShaderStructMetaInfo,
    device: &GPUDevice,
  ) -> Self {
    let usage = BufferUsages::STORAGE | BufferUsages::COPY_SRC;

    let stride = task_ty_desc.size_of_self(StructLayoutTarget::Std430);

    let init = BufferInit::Zeroed(NonZeroU64::new((size * stride) as u64).unwrap());
    let desc = GPUBufferDescriptor {
      size: init.size(),
      usage,
    };

    let gpu = GPUBuffer::create(device, init, usage);
    let gpu = GPUBufferResource::create_with_raw(gpu, desc, device).create_default_view();

    Self {
      tasks: gpu,
      state_desc,
      task_ty_desc,
    }
  }

  pub fn build_shader(&self, cx: &mut ShaderComputePipelineBuilder) -> TaskPoolInvocationInstance {
    let desc = ShaderBindingDescriptor {
      should_as_storage_buffer_if_is_buffer_like: true,
      ty: ShaderValueType::Single(ShaderValueSingleType::Unsized(
        ShaderUnSizedValueType::UnsizedArray(Box::new(ShaderSizedValueType::Struct(
          self.task_ty_desc.clone(),
        ))),
      )),
      writeable_if_storage: true,
    };
    let node = cx.bindgroups().binding_dyn(desc).compute_node;
    TaskPoolInvocationInstance {
      pool: unsafe { node.into_node() },
      state_desc: self.state_desc.clone(),
      payload_ty: self.task_ty_desc.fields[1].ty.clone(),
    }
  }

  pub fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind_dyn(
      self.tasks.get_binding_build_source(),
      &ShaderBindingDescriptor {
        should_as_storage_buffer_if_is_buffer_like: true,
        ty: ShaderValueType::Single(ShaderValueSingleType::Sized(ShaderSizedValueType::Struct(
          self.state_desc.ty.clone(),
        ))),
        writeable_if_storage: true,
      },
    );
  }
}

#[derive(Clone)]
pub struct TaskPoolInvocationInstance {
  pool: StorageNode<[AnyType]>,
  state_desc: DynamicTypeMetaInfo,
  payload_ty: ShaderSizedValueType,
}

pub const TASK_STATUE_FLAG_NOT_FINISHED: u32 = 1;
pub const TASK_STATUE_FLAG_FINISHED: u32 = 2;

impl TaskPoolInvocationInstance {
  pub fn access_item_ptr(&self, idx: Node<u32>) -> StorageNode<AnyType> {
    self.pool.index(idx)
  }

  pub fn poll_task_is_finished(&self, task_id: Node<u32>) -> Node<bool> {
    self
      .rw_is_finished(task_id)
      .load()
      .equals(TASK_STATUE_FLAG_FINISHED)
  }

  pub fn spawn_new_task_dyn(
    &self,
    at: Node<u32>,
    payload: Node<AnyType>,
    ty: &ShaderSizedValueType,
  ) {
    self.rw_is_finished(at).store(TASK_STATUE_FLAG_NOT_FINISHED);

    self.rw_payload_dyn(at).store(payload);

    assert_eq!(&self.payload_ty, ty);

    // write states with given init value
    let state_ptr = self.rw_states(at);
    for (i, v) in self.state_desc.fields_init.iter().enumerate() {
      unsafe {
        let state_field: StorageNode<AnyType> = index_access_field(state_ptr.handle(), i);
        if let Some(v) = v {
          state_field.store(
            v.to_shader_node_by_value(&self.state_desc.ty.fields[i].ty)
              .into_node(),
          );
        }
      };
    }
  }

  pub fn rw_is_finished(&self, task: Node<u32>) -> StorageNode<u32> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { index_access_field(item_ptr.handle(), 0) }
  }
  pub fn rw_payload<T: ShaderSizedValueNodeType>(&self, task: Node<u32>) -> StorageNode<T> {
    assert_eq!(self.payload_ty, T::sized_ty());
    let item_ptr = self.access_item_ptr(task);
    unsafe { index_access_field(item_ptr.handle(), 1) }
  }
  pub fn rw_payload_dyn(&self, task: Node<u32>) -> StorageNode<AnyType> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { index_access_field(item_ptr.handle(), 1) }
  }
  pub fn rw_states(&self, task: Node<u32>) -> StorageNode<AnyType> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { index_access_field(item_ptr.handle(), 2) }
  }
}
