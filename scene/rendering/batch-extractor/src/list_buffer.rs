use crate::*;

pub struct PersistSceneModelListBuffer {
  buffer: PersistSceneModelListBufferWithLength,
  host: Vec<RawEntityHandle>,
}

impl PersistSceneModelListBuffer {
  pub fn create_batch(&self) -> DeviceSceneModelRenderSubBatch {
    DeviceSceneModelRenderSubBatch {
      scene_models: Box::new(self.buffer.clone()),
      impl_select_id: unsafe { EntityHandle::from_raw(*self.host.first().unwrap()) },
    }
  }
  pub fn with_capacity(capacity: usize, alloc: &dyn AbstractStorageAllocator, gpu: &GPU) -> Self {
    let init_byte_size = (capacity + 1) * std::mem::size_of::<u32>();

    Self {
      buffer: PersistSceneModelListBufferWithLength {
        buffer: alloc.allocate_readonly(
          init_byte_size as u64,
          &gpu.device,
          Some("PersistSceneModelListBuffer"),
        ),
      },
      host: Vec::with_capacity(capacity),
    }
  }
}

/// the [0] store the real length
#[derive(Clone)]
struct PersistSceneModelListBufferWithLength {
  buffer: AbstractReadonlyStorageBuffer<[u32]>,
}

impl DeviceParallelCompute<Node<u32>> for PersistSceneModelListBufferWithLength {
  fn execute_and_expose(
    &self,
    _cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn DeviceInvocationComponent<Node<u32>>> {
    Box::new(self.clone())
  }

  fn result_size(&self) -> u32 {
    self.buffer.item_count() - 1
  }
}
impl DeviceParallelComputeIO<u32> for PersistSceneModelListBufferWithLength {}
impl ShaderHashProvider for PersistSceneModelListBufferWithLength {
  shader_hash_type_id! {}
}
impl DeviceInvocationComponent<Node<u32>> for PersistSceneModelListBufferWithLength {
  fn work_size(&self) -> Option<u32> {
    None
  }

  fn build_shader(
    &self,
    builder: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn DeviceInvocation<Node<u32>>> {
    struct Invocation {
      buffer: ShaderReadonlyPtrOf<[u32]>,
    }

    impl DeviceInvocation<Node<u32>> for Invocation {
      fn invocation_logic(&self, logic_global_id: Node<Vec3<u32>>) -> (Node<u32>, Node<bool>) {
        let idx = logic_global_id.x();
        let access_idx = idx + val(1);

        let array_len = self.buffer.array_length();
        let r = access_idx.less_than(array_len);

        let result = r.select_branched(|| self.buffer.index(access_idx).load(), || val(0_u32));

        (result, r)
      }

      fn invocation_size(&self) -> Node<Vec3<u32>> {
        (self.buffer.index(0).load(), val(0), val(0)).into()
      }
    }

    Box::new(Invocation {
      buffer: builder.bind_by(&self.buffer),
    })
  }

  fn bind_input(&self, builder: &mut BindingBuilder) {
    builder.bind(&self.buffer);
  }

  fn requested_workgroup_size(&self) -> Option<u32> {
    None
  }
}
