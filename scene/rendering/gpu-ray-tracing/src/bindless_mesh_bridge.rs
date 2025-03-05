use crate::*;

#[derive(Clone)]
pub struct BindlessMeshRtxAccessInvocation {
  base: BindlessMeshDispatcherBaseInvocation,
  sm_to_mesh: ShaderPtrOf<[u32]>,
  indices: ShaderPtrOf<[u32]>,
}

impl BindlessMeshRtxAccessInvocation {
  pub fn get_triangle_idx(&self, primitive_id: Node<u32>, mesh_id: Node<u32>) -> Node<Vec3<u32>> {
    let vertex_id = primitive_id * val(3);

    let meta = self.base.vertex_address_buffer.index(mesh_id);
    let index_offset = meta.index_offset().load();

    let offset = index_offset + vertex_id;
    (
      self.indices.index(offset).load(),
      self.indices.index(offset + val(1)).load(),
      self.indices.index(offset + val(2)).load(),
    )
      .into()
  }

  pub fn get_world_normal(&self, closest_hit_ctx: &dyn ClosestHitCtxProvider) -> Node<Vec3<f32>> {
    let scene_model_id = closest_hit_ctx.instance_custom_id();
    let mesh_id = self.sm_to_mesh.index(scene_model_id).load();
    let tri_id = closest_hit_ctx.primitive_id();
    let tri_idx_s = self.get_triangle_idx(tri_id, mesh_id);

    let tri_a_normal = self.base.get_normal(mesh_id, tri_idx_s.x());
    let tri_b_normal = self.base.get_normal(mesh_id, tri_idx_s.y());
    let tri_c_normal = self.base.get_normal(mesh_id, tri_idx_s.z());

    let attribs: Node<Vec2<f32>> = closest_hit_ctx.hit_attribute().expand().bary_coord;
    let barycentric: Node<Vec3<f32>> = (
      val(1.0) - attribs.x() - attribs.y(),
      attribs.x(),
      attribs.y(),
    )
      .into();

    // Computing the normal at hit position
    let normal = tri_a_normal * barycentric.x()
      + tri_b_normal * barycentric.y()
      + tri_c_normal * barycentric.z();
    // Transforming the normal to world space
    (closest_hit_ctx.world_to_object().shrink_to_3().transpose() * normal).normalize()
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
      indices: cx.bind_by(&self.index_pool),
      sm_to_mesh: cx.bind_by(&self.sm_to_mesh),
      base: self.build_base_invocation(cx),
    }
  }

  fn bind_bindless_mesh_rtx_access(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.index_pool);
    cx.bind(&self.sm_to_mesh);
    self.bind_base_invocation(cx);
  }
}
