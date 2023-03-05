pub trait Animatable {
  fn animate(&mut self, time: f32);
}

pub trait InterpolateAble: Sized {
  fn interpolate(self, target: Self, normalized: f32) -> Option<Self>;
}

pub trait KeyframeTrack {
  type Value;
  fn sample_animation(&mut self, time: f32) -> Option<Self::Value>;
}
