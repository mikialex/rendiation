use crate::*;

#[derive(Clone)]
pub struct BindlessMeshRtxAccessInvocation {
  normal: ShaderPtrOf<[u32]>,
  indices: ShaderPtrOf<[u32]>,
  address: ShaderPtrOf<[AttributeMeshMeta]>,
  pub sm_to_mesh: ShaderPtrOf<[u32]>,
}

impl BindlessMeshRtxAccessInvocation {
  pub fn get_triangle_idx(&self, primitive_id: Node<u32>, mesh_id: Node<u32>) -> Node<Vec3<u32>> {
    let vertex_id = primitive_id * val(3);
    let index_offset = self.address.index(mesh_id).index_offset().load();
    let offset = index_offset + vertex_id;
    (
      self.indices.index(offset).load(),
      self.indices.index(offset + val(1)).load(),
      self.indices.index(offset + val(2)).load(),
    )
      .into()
  }

  pub fn get_normal(&self, index: Node<u32>, mesh_id: Node<u32>) -> Node<Vec3<f32>> {
    let normal_offset = self.address.index(mesh_id).normal_offset().load();

    unsafe {
      Vec3::<f32>::sized_ty()
        .load_from_u32_buffer(
          &self.normal,
          normal_offset + index * val(3),
          StructLayoutTarget::Packed,
        )
        .into_node::<Vec3<f32>>()
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
      sm_to_mesh: cx.bind_by(&self.sm_to_mesh),
    }
  }

  fn bind_bindless_mesh_rtx_access(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.normal);
    cx.bind(&self.index_pool);
    cx.bind(&self.vertex_address_buffer);
    cx.bind(&self.sm_to_mesh);
  }
}
