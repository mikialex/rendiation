pub struct DeviceSceneRepresentation<T> {
  adaptor: T,
  models: StorageBuffer<ShaderSceneModelInfo>,
  nodes: StorageBuffer<ShaderNodeInfo>,
  meshes: StorageBuffer<DrawIndexedIndirect>,

  material_a: StorageBuffer<MaterialA>,
  material_b: StorageBuffer<MaterialB>,

  lod_mesh: StorageBuffer<LODMetaData>,
  common_mesh: StorageBuffer<DrawIndexedIndirect>,
}

impl<T: SceneRasterRenderingAdaptor> SceneRasterRenderingAdaptor for DeviceSceneRepresentation<T> {}

// maintained by cpu side
pub struct ShaderSceneModelInfo {
  pub material_idx: u32,
  pub material_type_idx: u32,
  pub mesh_idx: u32,
  pub mesh_type_idx: u32,
  pub node_idx: StorageArrayHandle<ShaderNodeInfo>,
  pub world_aabb: ShaderAABB,
}

// maintained by cpu side
pub struct ShaderNodeInfo {
  pub world_mat: Mat4<f32>,
  pub filter_flags: u32,
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
