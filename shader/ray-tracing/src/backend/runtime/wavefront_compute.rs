use crate::*;

pub struct WavefrontExecutor {
  stages: Vec<WavefrontStageExecutor>,
  max_recursion_depth: u32,
  current_prepared_execution_size: (u32, u32, u32),
}

pub struct WavefrontExecutorBuildCtx {
  state_builder: DynamicStateBuilder,
}

pub struct DynamicStateBuilder {
  state: Vec<(PrimitiveShaderValueType, PrimitiveShaderValue)>,
  node_to_resolve: Arc<RwLock<Option<NodeUntyped>>>,
}

impl DeviceStateProvider for DynamicStateBuilder {
  fn create_or_reconstruct_inline_state<T: PrimitiveShaderNodeType>(
    &mut self,
    default: T,
  ) -> BoxedShaderLoadStore<Node<T>> {
    let field_index = self.state.len();
    self.state.push((T::PRIMITIVE_TYPE, default.to_primitive()));

    let node = DeferResolvedStorageStructFieldNode {
      node: Arc::downgrade(&self.node_to_resolve),
      field_index: field_index as u32,
      resolved_node: Default::default(),
    };

    Box::new(node)
  }
}

struct DeferResolvedStorageStructFieldNode {
  node: Weak<RwLock<Option<NodeUntyped>>>,
  field_index: u32,
  resolved_node: RwLock<Option<NodeUntyped>>,
}
impl<T: PrimitiveShaderNodeType> ShaderAbstractLoadStore<Node<T>>
  for DeferResolvedStorageStructFieldNode
{
  fn abstract_load(&self) -> Node<T> {
    //  self.resolved_node.
    todo!()
  }

  fn abstract_store(&self, payload: Node<T>) {
    todo!()
  }
}

impl DeviceStateProvider for WavefrontExecutorBuildCtx {
  fn create_or_reconstruct_inline_state<T: PrimitiveShaderNodeType>(
    &mut self,
    default: T,
  ) -> BoxedShaderLoadStore<Node<T>> {
    self
      .state_builder
      .create_or_reconstruct_inline_state(default)
  }
}

impl WavefrontExecutor {
  fn empty() -> Self {
    Self {
      stages: Default::default(),
      max_recursion_depth: 6,
      current_prepared_execution_size: (1, 1, 1),
    }
  }

  fn define_state<F>(
    &self,
    future: F,
    cx_provider: impl FnOnce(&mut WavefrontExecutorBuildCtx) -> F::Ctx,
  ) -> u32
  where
    F: DeviceFuture,
  {
    todo!()
  }
}

impl WavefrontExecutor {
  pub fn compile_from(desc: &GPURaytracingPipelineBuilder) -> Self {
    let mut executor = Self::empty();
    executor
  }
  pub fn set_execution_size(&mut self, gpu: &GPU, dispatch_size: (u32, u32, u32)) {
    let dispatch_size = (
      dispatch_size.0.min(1),
      dispatch_size.1.min(1),
      dispatch_size.2.min(1),
    );
    if self.current_prepared_execution_size == dispatch_size {
      return;
    }
    self.current_prepared_execution_size = dispatch_size;
    todo!()
  }

  fn make_sure_execution_size_is_enough(&mut self, gpu: &GPU, dispatch_size: (u32, u32, u32)) {
    let is_contained = self.current_prepared_execution_size.0 <= dispatch_size.0
      && self.current_prepared_execution_size.1 <= dispatch_size.1
      && self.current_prepared_execution_size.2 <= dispatch_size.2;

    if !is_contained {
      self.set_execution_size(gpu, dispatch_size)
    }
  }
}

impl WavefrontExecutor {
  pub fn execute(&mut self, gpu: &GPU, dispatch_size: (u32, u32, u32)) {
    self.make_sure_execution_size_is_enough(gpu, dispatch_size);

    let mut encoder = gpu.create_encoder();

    encoder.compute_pass_scoped(|pass| {
      for _ in 0..self.max_recursion_depth {
        for stage in &self.stages {
          // pass.dispatch_workgroups_indirect(indirect_buffer, indirect_offset)
        }
      }
    });
    // todo check state states to make sure no task remains
  }
}

struct WavefrontStageExecutor {
  index: usize,
  depend_on: Vec<usize>,
  depend_by: Vec<usize>,
  task: GPUBufferView, // (task_state, payload)
  batch_info: GPUBufferView,
  pipeline: GPUComputePipeline,
}
