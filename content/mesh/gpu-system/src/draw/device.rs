use crate::*;

impl GPUBindlessMeshSystem {
  pub fn create_device_draw_dispatcher(&self, device: &GPUDevice) -> BindlessDrawCreator {
    let inner = self.inner.read().unwrap();
    let metadata = slab_to_vec(&inner.metadata);
    let metadata = StorageBufferReadOnlyDataView::create(device, metadata.as_slice());
    BindlessDrawCreator { metadata }
  }
}

pub struct BindlessDrawCreator {
  metadata: StorageBufferReadOnlyDataView<[DrawMetaData]>,
}

impl BindlessDrawCreator {
  pub fn setup_pass(&self, binding: &mut BindingBuilder) {
    binding.bind(&self.metadata);
  }

  pub fn register_shader_resource(&self, cx: &mut ComputeCx) -> BindlessDrawCreatorInDevice {
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
