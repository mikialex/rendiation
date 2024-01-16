use crate::*;

pub struct LODMetaData {
  pub first_level: StorageArrayHandle<LODLevelInfo>,
}

pub struct LODLevelInfo {
  pub info: usize,
  pub reference_mesh: StorageArrayHandle<DrawIndexedIndirect>,
  pub next_level: StorageArrayHandle<Self>,
}
