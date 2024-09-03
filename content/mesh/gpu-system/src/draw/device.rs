use crate::*;

impl GPUBindlessMeshSystem {
  pub fn create_device_draw_dispatcher(&self, device: &GPUDevice) -> BindlessDrawCreator {
    let metadata = slab_to_vec(&self.metadata);
    let metadata = StorageBufferReadOnlyDataView::create(device, metadata.as_slice());
    BindlessDrawCreator { metadata }
  }
}

// this is not good, maybe we should impl slab by ourself?
fn slab_to_vec<T: Clone>(s: &Slab<T>) -> Vec<T> {
  let mut r = Vec::with_capacity(s.capacity());
  let default = s.get(0).unwrap();
  s.iter().for_each(|(idx, v)| {
    while idx >= r.len() {
      r.push(default.clone())
    }
    r[idx] = v.clone();
  });
  r
}

pub struct BindlessDrawCreator {
  metadata: StorageBufferReadOnlyDataView<[DrawMetaData]>,
}

impl BindlessDrawCreator {
  pub fn setup_pass(&self, binding: &mut BindingBuilder) {
    binding.bind(&self.metadata);
  }

  pub fn register_shader_resource(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> BindlessDrawCreatorInDevice {
    let node = cx.bind_by(&self.metadata);
    BindlessDrawCreatorInDevice { node }
  }
}

pub struct BindlessDrawCreatorInDevice {
  node: ReadOnlyStorageNode<[DrawMetaData]>,
}

impl BindlessDrawCreatorInDevice {
  pub fn generate_draw_command(
    &self,
    mesh_handle: Node<u32>,
    draw_id: Node<u32>,
  ) -> (Node<DrawIndexedIndirect>, Node<DrawVertexIndirectInfo>) {
    let meta = self.node.index(mesh_handle).load().expand();
    let draw = ENode::<DrawIndexedIndirect> {
      vertex_count: meta.count,
      instance_count: val(1),
      base_index: meta.start,
      vertex_offset: val(0),
      base_instance: draw_id,
    }
    .construct();

    (draw, meta.vertex_info)
  }
}
