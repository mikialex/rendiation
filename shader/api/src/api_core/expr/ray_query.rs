use rendiation_algebra::{Mat4x3, Vec2, Vec3};

use crate::{
  call_shader_api, index_access_field, AnyType, BindingNode, Node, ShaderAccelerationStructure,
  ShaderNodeExpr, ShaderNodeRawHandle, ShaderRayDesc, ShaderRayQuery, ShaderValueSingleType,
  ShaderValueType,
};

impl Node<ShaderRayQuery> {
  pub fn new() -> Self {
    call_shader_api(|api| unsafe {
      api
        .make_local_var(ShaderValueType::Single(ShaderValueSingleType::RayQuery))
        .into_node()
    })
  }
  pub fn initialize(
    self,
    tlas: BindingNode<ShaderAccelerationStructure>,
    flags: Node<u32>,
    cull_mask: Node<u32>,
    t_min: Node<f32>,
    t_max: Node<f32>,
    origin: Node<Vec3<f32>>,
    dir: Node<Vec3<f32>>,
  ) {
    call_shader_api(|api| {
      api.ray_query_initialize(
        self.handle(),
        tlas,
        ShaderRayDesc {
          flags: flags.handle(),
          cull_mask: cull_mask.handle(),
          t_min: t_min.handle(),
          t_max: t_max.handle(),
          origin: origin.handle(),
          dir: dir.handle(),
        },
      )
    })
  }
  pub fn terminate(self) {
    call_shader_api(|api| api.ray_query_terminate(self.handle()))
  }

  pub fn proceed(self) -> Node<bool> {
    ShaderNodeExpr::RayQueryProceed {
      ray_query: self.handle(),
    }
    .insert_api()
  }
  pub fn get_candidate_intersection(self) -> RayIntersection {
    let node: Node<AnyType> = ShaderNodeExpr::RayQueryGetCandidateIntersection {
      ray_query: self.handle(),
    }
    .insert_api();
    RayIntersection { raw: node.handle() }
  }
  pub fn get_commited_intersection(self) -> RayIntersection {
    let node: Node<AnyType> = ShaderNodeExpr::RayQueryGetCommitedIntersection {
      ray_query: self.handle(),
    }
    .insert_api();
    RayIntersection { raw: node.handle() }
  }
  // todo confirm hit
}

// struct RayIntersection {
//   kind: u32,
//   t: f32,
//   instance_custom_index: u32,
//   instance_id: u32,
//   sbt_record_offset: u32,
//   geometry_index: u32,
//   primitive_index: u32,
//   barycentrics: vec2<f32>,
//   front_face: bool,
//   object_to_world: mat4x3<f32>,
//   world_to_object: mat4x3<f32>,
// }
#[derive(Copy, Clone)]
pub struct RayIntersection {
  raw: ShaderNodeRawHandle,
}
impl RayIntersection {
  pub fn kind(self) -> Node<u32> {
    unsafe { index_access_field(self.raw, 0).into_node() }
  }
  pub fn t(self) -> Node<f32> {
    unsafe { index_access_field(self.raw, 1).into_node() }
  }
  pub fn instance_custom_index(self) -> Node<u32> {
    unsafe { index_access_field(self.raw, 2).into_node() }
  }
  pub fn instance_id(self) -> Node<u32> {
    unsafe { index_access_field(self.raw, 3).into_node() }
  }
  pub fn sbt_record_offset(self) -> Node<u32> {
    unsafe { index_access_field(self.raw, 4).into_node() }
  }
  pub fn geometry_index(self) -> Node<u32> {
    unsafe { index_access_field(self.raw, 5).into_node() }
  }
  pub fn primitive_index(self) -> Node<u32> {
    unsafe { index_access_field(self.raw, 6).into_node() }
  }
  pub fn barycentrics(self) -> Node<Vec2<f32>> {
    unsafe { index_access_field(self.raw, 7).into_node() }
  }
  pub fn front_face(self) -> Node<bool> {
    unsafe { index_access_field(self.raw, 8).into_node() }
  }
  pub fn object_to_world(self) -> Node<Mat4x3<f32>> {
    unsafe { index_access_field(self.raw, 9).into_node() }
  }
  pub fn world_to_object(self) -> Node<Mat4x3<f32>> {
    unsafe { index_access_field(self.raw, 10).into_node() }
  }
}

#[repr(u32)]
pub enum RayIntersectionKind {
  None = 0,
  Triangle = 1,
  Generated = 2,
  AABB = 3,
}
