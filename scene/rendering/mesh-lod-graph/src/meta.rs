use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct, PartialEq)]
pub struct MeshletMeshMetaData {
  pub meshlet_count: u32,
  pub global_meshlet_data_offset: u32,
  pub global_index_buffer_offset: u32,
  pub global_position_buffer_offset: u32,
}

impl Default for MeshletMeshMetaData {
  fn default() -> Self {
    Self {
      meshlet_count: u32::MAX,
      global_meshlet_data_offset: u32::MAX,
      global_index_buffer_offset: u32::MAX,
      global_position_buffer_offset: u32::MAX,
      ..Zeroable::zeroed()
    }
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct, PartialEq)]
pub struct MeshletData {
  pub index_start: u32,
  pub index_end: u32,
  pub self_group_index: u32,
  pub level_index: u32,
  pub position_buffer_base_offset: i32,
  pub bounds: LODBoundPair,
}
