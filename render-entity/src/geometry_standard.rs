pub struct StandardGeometry {
  pub bounding_box: Box3,
  pub bounding_sphere: Sphere,

  pub index: Rc<BufferData<u16>>,
  pub position: BufferData<f32>,
  // pub normal: BufferData<f32>,
  // pub uv: BufferData<f32>,
}
