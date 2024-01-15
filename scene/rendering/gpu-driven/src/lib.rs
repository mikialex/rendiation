pub struct DeviceSceneRepresentation {
  models: StorageBuffer<ShaderSceneModelInfo>,
  nodes: StorageBuffer<ShaderNodeInfo>,
  meshes: StorageBuffer<DrawIndexedIndirect>,

  material_a: StorageBuffer<MaterialA>,
  material_b: StorageBuffer<MaterialB>,
}

pub struct LODMetaData {
  first_level: StorageArrayHandle<LODLevelInfo>,
}

pub struct LODLevelInfo {
  info: usize,
  reference_mesh: StorageArrayHandle<DrawIndexedIndirect>,
  next_level: StorageArrayHandle<Self>,
}

// maintained by cpu side
pub struct ShaderSceneModelInfo {
  pub material_idx: u32, // untyped to reduce code bloat
  pub mesh_idx: StorageArrayHandle<DrawIndexedIndirect>,
  pub node_idx: StorageArrayHandle<ShaderNodeInfo>,
  pub world_aabb: ShaderAABB,
}

// maintained by cpu side
pub struct ShaderNodeInfo {
  pub world_mat: Mat4<f32>,
}

// not retained
pub struct DrawCommandBuffer {
  model_idx: StorageArrayHandle<ShaderSceneModelInfo>,
}

pub fn update_gpu_storage<T>(
  buffer: GPUStorageBuffer<[T]>,
  source: impl ReactiveCollection<usize, T>,
) {
  //
}
