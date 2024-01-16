pub struct LODMetaData {
  first_level: StorageArrayHandle<LODLevelInfo>,
}

pub struct LODLevelInfo {
  info: usize,
  reference_mesh: StorageArrayHandle<DrawIndexedIndirect>,
  next_level: StorageArrayHandle<Self>,
}
