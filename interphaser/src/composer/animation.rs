use std::time::Duration;

use crate::UpdateCtx;

pub trait TransitionAnimator<T> {
  fn update(&self, current: &mut T, target: &T, last_frame_delta: Duration);
}

pub struct TransitionAnimation<A, T> {
  animator: A,
  pair: Option<Pair<T>>,
}

struct Pair<T> {
  current: T,
  target: T,
}

impl<A: TransitionAnimator<T>, T: Clone> TransitionAnimation<A, T> {
  pub fn update(&mut self, new: T, ctx: &mut UpdateCtx) -> &T {
    let Pair { current, target } = self.pair.get_or_insert_with(|| Pair {
      current: new.clone(),
      target: new,
    });

    self
      .animator
      .update(current, target, ctx.last_frame_perf_info.all_time);

    current
  }
}

pub enum Transition {
  Linear,
  Cubic {
    //
  },
}

pub struct TimeBasedTransition {
  duration: Duration,
  ty: Transition,
}