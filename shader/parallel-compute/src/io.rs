use crate::*;

impl<T: ShaderNodeType> DeviceInvocation<T> for Node<ShaderReadOnlyStoragePtr<[T]>> {
  fn invocation_logic(&self, cx: ComputeCx) -> (Node<T>, Node<bool>) {
    let idx = cx.global_invocation_id().x();
    let r = idx.less_than(self.array_length());
    (r.select(self.index(idx).load(), zero_shader_value()), r)
  }
}

impl<V: Std430 + ShaderSizedValueNodeType> DeviceParallelCompute<V>
  for StorageBufferReadOnlyDataView<[V]>
{
  fn build_invocation_access(
    &self,
    _cx: &mut DeviceParallelComputeCtx,
  ) -> Box<dyn FnOnce(&mut ShaderComputePipelineBuilder) -> Box<dyn DeviceInvocation<V>>> {
    let data = self.clone();
    let logic = move |builder: &mut ShaderComputePipelineBuilder| {
      let view = builder.entry_by(|cx| cx.bind_by(&data));
      Box::new(view) as Box<dyn DeviceInvocation<V>>
    };
    Box::new(logic)
  }
}
