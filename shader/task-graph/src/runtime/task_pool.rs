use crate::*;

#[derive(Clone)]
pub struct TaskPool {
  /// struct Task {
  ///   is_finished_ bool,
  ///   payload: P,
  ///   state: S,
  ///   parent_task_type_id: u32,
  ///   parent_task_index: u32,
  /// }
  pub(crate) tasks: GPUBufferResourceView,
  state_desc: DynamicTypeMetaInfo,
  task_ty_desc: ShaderStructMetaInfo,
}

impl ShaderHashProvider for TaskPool {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.task_ty_desc.hash(hasher);
  }

  shader_hash_type_id! {}
}

impl TaskPool {
  pub fn create_with_size(
    size: usize,
    state_desc: DynamicTypeMetaInfo,
    payload_ty: ShaderSizedValueType,
    device: &GPUDevice,
  ) -> Self {
    let usage = BufferUsages::STORAGE | BufferUsages::COPY_SRC;

    let mut task_ty_desc = ShaderStructMetaInfo::new("TaskType");
    let u32_ty = ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Uint32);

    task_ty_desc.push_field_dyn("is_finished", u32_ty.clone());
    task_ty_desc.push_field_dyn("payload", payload_ty);
    task_ty_desc.push_field_dyn("state", ShaderSizedValueType::Struct(state_desc.ty.clone()));
    task_ty_desc.push_field_dyn("parent_task_type_id", u32_ty.clone());
    task_ty_desc.push_field_dyn("parent_task_index", u32_ty);

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
    cx.bind_dyn(self.tasks.get_binding_build_source());
  }
}

#[derive(Clone)]
pub struct TaskPoolInvocationInstance {
  pool: StorageNode<[AnyType]>,
  state_desc: DynamicTypeMetaInfo,
  payload_ty: ShaderSizedValueType,
}

pub const TASK_STATUE_FLAG_TASK_NOT_EXIST: u32 = 0;
pub const TASK_STATUE_FLAG_NOT_FINISHED_WAKEN: u32 = 1;
pub const TASK_STATUE_FLAG_NOT_FINISHED_SLEEP: u32 = 2;
pub const TASK_STATUE_FLAG_FINISHED: u32 = 3;

#[derive(Clone, Copy)]
pub struct TaskParentRef {
  pub parent_task_index: Node<u32>,
  pub parent_task_type_id: Node<u32>,
}

impl TaskParentRef {
  pub fn none_parent() -> Self {
    Self {
      parent_task_index: val(u32::MAX),
      parent_task_type_id: val(u32::MAX),
    }
  }
}

impl TaskPoolInvocationInstance {
  pub fn access_item_ptr(&self, idx: Node<u32>) -> StorageNode<AnyType> {
    self.pool.index(idx)
  }

  pub fn is_task_finished(&self, task_id: Node<u32>) -> Node<bool> {
    self
      .rw_is_finished(task_id)
      .load()
      .equals(TASK_STATUE_FLAG_FINISHED)
  }

  pub fn is_task_unfinished_waken(&self, task_id: Node<u32>) -> Node<bool> {
    self
      .rw_is_finished(task_id)
      .load()
      .equals(TASK_STATUE_FLAG_NOT_FINISHED_WAKEN)
  }

  pub fn spawn_new_task_dyn(
    &self,
    at: Node<u32>,
    payload: Node<AnyType>,
    parent_ref: TaskParentRef,
    ty: &ShaderSizedValueType,
  ) {
    self
      .rw_is_finished(at)
      .store(TASK_STATUE_FLAG_NOT_FINISHED_WAKEN);

    self
      .rw_parent_task_index(at)
      .store(parent_ref.parent_task_index);
    self
      .rw_parent_task_type_id(at)
      .store(parent_ref.parent_task_type_id);

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

  pub fn rw_parent_task_type_id(&self, task: Node<u32>) -> StorageNode<u32> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { index_access_field(item_ptr.handle(), 3) }
  }
  pub fn rw_parent_task_index(&self, task: Node<u32>) -> StorageNode<u32> {
    let item_ptr = self.access_item_ptr(task);
    unsafe { index_access_field(item_ptr.handle(), 4) }
  }
}
