use crate::UpdateCtx;

pub type Millisecond = u32;

pub trait Animator<T> {
  fn update(&mut self, new: T, last_frame_delta: Millisecond);
}

pub enum Transition {
  Linear,
}

impl Transition {
  fn transit(&self, normalized: f32) -> f32 {
    match self {
      Transition::Linear => normalized,
    }
  }
}

pub struct TimeBasedTransition {
  pub duration: Millisecond, // ms
  pub ty: Transition,
}

impl TimeBasedTransition {
  pub fn into_animation<T>(self) -> TimeBasedTransitionInstance<T> {
    TimeBasedTransitionInstance {
      config: self,
      used_time: 0,
      pair: None,
    }
  }
}

pub struct TimeBasedTransitionInstance<T> {
  config: TimeBasedTransition,
  used_time: Millisecond,
  pair: Option<Pair<T>>,
}

struct Pair<T> {
  start: T,
  target: T,
}

pub trait AnimationInterpolateAble: Sized {
  fn interpolate(start: &Self, target: &Self, normalized: f32) -> Self;
}

impl<T: Clone + PartialEq + AnimationInterpolateAble> TimeBasedTransitionInstance<T> {
  pub fn update(&mut self, new: T, ctx: &mut UpdateCtx) -> T {
    let Pair { start, target } = self.pair.get_or_insert_with(|| Pair {
      start: new.clone(),
      target: new.clone(),
    });

    if target.clone() != new {
      self.used_time = 0;
      *target = new;
    }

    if self.used_time == self.config.duration {
      return target.clone();
    }

    let delta = ctx.last_frame_perf_info.all_time.as_millis() as Millisecond;

    self.used_time += delta;
    self.used_time = self.used_time.min(self.config.duration);

    let normalized_end = self.used_time as f32 / self.config.duration as f32;
    let normalized_end = self.config.ty.transit(normalized_end);

    T::interpolate(start, target, normalized_end)
  }
}
