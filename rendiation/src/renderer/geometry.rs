struct GPUGeometry{
  geometry: StandardGeometry,
  data_changed: bool,
  index_changed: bool,
  gpu_data: Option<WGPUBuffer>,
  gpu_index: Option<WGPUBuffer>,
}