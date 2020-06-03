
use rendiation_math::Vec2;

#[derive(Debug, Copy, Clone)]
pub struct Box2<T = f32> {
  pub min: Vec2<T>,
  pub max: Vec2<T>,
}