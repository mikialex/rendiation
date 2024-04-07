use crate::*;

pub fn transform_instance_buffer(
  _cx: &GPUResourceCtx,
) -> impl ReactiveCollection<AllocIdx<InstanceMeshInstanceEntity>, GPUBufferResourceView> {
  global_rev_ref().watch_inv_ref::<InstanceMeshInstanceEntityRefAttributeMesh>();
  global_watch().watch_typed_key::<InstanceMeshWorldMatrix>();
}
