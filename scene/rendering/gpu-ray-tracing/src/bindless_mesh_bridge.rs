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

  fn get_data_accessor(
    &self,
    closest_hit_ctx: &dyn ClosestHitCtxProvider,
  ) -> (Node<Vec3<u32>>, Node<Vec3<f32>>, Node<u32>) {
    let scene_model_id = closest_hit_ctx.instance_custom_id();
    let mesh_id = self.sm_to_mesh.index(scene_model_id).load();
    let tri_id = closest_hit_ctx.primitive_id();
    let tri_idx_s = self.get_triangle_idx(tri_id, mesh_id);

    let attribs: Node<Vec2<f32>> = closest_hit_ctx.hit_attribute().expand().bary_coord;
    let barycentric: Node<Vec3<f32>> = (
      val(1.0) - attribs.x() - attribs.y(),
      attribs.x(),
      attribs.y(),
    )
      .into();

    (tri_idx_s, barycentric, mesh_id)
  }

  pub fn get_uv(&self, closest_hit_ctx: &dyn ClosestHitCtxProvider) -> Node<Vec2<f32>> {
    self.get_uv_impl(self.get_data_accessor(closest_hit_ctx))
  }

  pub fn get_uv_impl(
    &self,
    (tri_idx_s, barycentric, mesh_id): (Node<Vec3<u32>>, Node<Vec3<f32>>, Node<u32>),
  ) -> Node<Vec2<f32>> {
    let tri_a_uv = self.base.get_uv(mesh_id, tri_idx_s.x());
    let tri_b_uv = self.base.get_uv(mesh_id, tri_idx_s.y());
    let tri_c_uv = self.base.get_uv(mesh_id, tri_idx_s.z());

    tri_a_uv * barycentric.x() + tri_b_uv * barycentric.y() + tri_c_uv * barycentric.z()
  }

  /// return (shading, geom)
  pub fn get_world_normal(
    &self,
    closest_hit_ctx: &dyn ClosestHitCtxProvider,
  ) -> (Node<Vec3<f32>>, Node<Vec3<f32>>) {
    self.get_world_normal_impl(closest_hit_ctx, self.get_data_accessor(closest_hit_ctx))
  }

  pub fn get_world_normal_impl(
    &self,
    closest_hit_ctx: &dyn ClosestHitCtxProvider,
    (tri_idx_s, barycentric, mesh_id): (Node<Vec3<u32>>, Node<Vec3<f32>>, Node<u32>),
  ) -> (Node<Vec3<f32>>, Node<Vec3<f32>>) {
    let tri_a_normal = self.base.get_normal(mesh_id, tri_idx_s.x());
    let tri_b_normal = self.base.get_normal(mesh_id, tri_idx_s.y());
    let tri_c_normal = self.base.get_normal(mesh_id, tri_idx_s.z());

    let normal_mat = closest_hit_ctx.world_to_object().shrink_to_3().transpose();

    // Computing the normal at hit position
    let normal = tri_a_normal * barycentric.x()
      + tri_b_normal * barycentric.y()
      + tri_c_normal * barycentric.z();
    let shading_normal = (normal_mat * normal).normalize();

    let p_a = self.base.get_position(mesh_id, tri_idx_s.x());
    let p_b = self.base.get_position(mesh_id, tri_idx_s.y());
    let p_c = self.base.get_position(mesh_id, tri_idx_s.z());

    let geom_normal = (normal_mat * (p_a - p_b).cross(p_a - p_c)).normalize();

    // make sure the normal is towards the incoming ray
    let hit_to_origin = closest_hit_ctx.world_ray().origin - closest_hit_ctx.hit_world_position();
    let geom_normal = hit_to_origin
      .dot(geom_normal)
      .less_than(0.)
      .select(-geom_normal, geom_normal);

    // if the shading normal direction is different from the geometry normal, reverse the shading normal
    let shading_normal = geom_normal
      .dot(shading_normal)
      .less_than(0.)
      .select(-shading_normal, shading_normal);

    (shading_normal, geom_normal)
  }

  /// return (shading, geom, uv)
  pub fn get_world_normal_and_uv(
    &self,
    closest_hit_ctx: &dyn ClosestHitCtxProvider,
  ) -> (Node<Vec3<f32>>, Node<Vec3<f32>>, Node<Vec2<f32>>) {
    let acc = self.get_data_accessor(closest_hit_ctx);
    let uv = self.get_uv_impl(acc);
    let (s, g) = self.get_world_normal_impl(closest_hit_ctx, acc);
    (s, g, uv)
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
