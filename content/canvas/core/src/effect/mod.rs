pub enum CanvasEffect {
  Blur(BlurEffect),
}

pub struct BlurEffect {
  pub radius: f32,
}
