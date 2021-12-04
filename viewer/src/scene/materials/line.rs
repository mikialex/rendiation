use rendiation_algebra::Vec3;

#[derive(Clone)]
pub struct LineMaterial {
  pub color: Vec3<f32>,
}

#[derive(Clone)]
pub struct LineDash {
  pub screen_spaced: bool,
  pub scale: f32,
  pub gap_size: f32,
  pub dash_size: f32,
  pub view_scale: f32,
}
