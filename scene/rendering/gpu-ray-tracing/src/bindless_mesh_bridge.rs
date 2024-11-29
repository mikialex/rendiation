use crate::*;

#[derive(Clone)]
pub struct BindlessMeshRtxAccessInvocation {
  normal: StorageNode<[u32]>,
  indices: StorageNode<[u32]>,
  address: StorageNode<[AttributeMeshMeta]>,
}

impl BindlessMeshRtxAccessInvocation {
  pub fn get_triangle_idx(&self, primitive_id: Node<u32>, mesh_id: Node<u32>) -> Node<Vec3<u32>> {
    let vertex_id = primitive_id * val(3);
    let index_offset = self.address.index(mesh_id).load().expand().index_offset; // todo fix full expand
    let offset = index_offset + vertex_id;
    (
      self.indices.index(offset).load(),
      self.indices.index(offset + val(1)).load(),
      self.indices.index(offset + val(2)).load(),
    )
      .into()
  }

  pub fn get_normal(&self, index: Node<u32>, mesh_id: Node<u32>) -> Node<Vec3<f32>> {
    let normal_offset = self.address.index(mesh_id).load().expand().normal_offset; // todo fix full expand

    unsafe {
      Vec3::<f32>::sized_ty()
        .load_from_u32_buffer(self.normal, normal_offset + index * val(3))
        .cast_type::<Vec3<f32>>()
    }
  }
}

pub trait BindlessMeshDispatcherRtxEXT {
  fn build_bindless_mesh_rtx_access(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> BindlessMeshRtxAccessInvocation;
  fn bind_bindless_mesh_rtx_access(&self, cx: &mut BindingBuilder);
}

impl BindlessMeshDispatcherRtxEXT for BindlessMeshDispatcher {
  fn build_bindless_mesh_rtx_access(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> BindlessMeshRtxAccessInvocation {
    BindlessMeshRtxAccessInvocation {
      normal: cx.bind_by(&self.normal),
      indices: cx.bind_by(&self.index_pool),
      address: cx.bind_by(&self.vertex_address_buffer),
    }
  }

  fn bind_bindless_mesh_rtx_access(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.normal);
    cx.bind(&self.index_pool);
    cx.bind(&self.vertex_address_buffer);
  }
}
