use crate::*;

pub struct HitGroupShaderRecord {
  closet_hit: ShaderHandle,
  any_hit: Option<ShaderHandle>,
  intersection: Option<ShaderHandle>,
}

pub struct ShaderBindingTable {
  pub ray_generation: ShaderHandle,
  pub ray_miss: Vec<ShaderHandle>,        // ray_type_count size
  pub ray_hit: Vec<HitGroupShaderRecord>, // mesh_count size
}

impl ShaderBindingTable {
  pub fn new(mesh_count: u32, ray_type_count: u32) -> Self {
    ShaderBindingTable {
      ray_generation: todo!(),
      ray_miss: todo!(),
      ray_hit: todo!(),
    }
  }

  pub fn resize(&mut self, mesh_count: u32, ray_type_count: u32) {
    todo!()
  }

  pub fn config_ray_generation(&mut self, s: ShaderHandle) -> &mut Self {
    self
  }
  pub fn config_hit_group(&mut self, mesh_idx: u32, hit_group: HitGroupShaderRecord) -> &mut Self {
    self
  }
  pub fn config_missing(&mut self, ray_ty_idx: u32, s: ShaderHandle) -> &mut Self {
    self
  }
}
