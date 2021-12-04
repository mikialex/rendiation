use crate::UpdateCtx;

pub trait TransitionAnimator<T> {
  fn update(&self, current: &mut T, target: &T, frame_delta: f32);
}

pub struct TransitionAnimation<A, T> {
  animator: A,
  current: T,
  target: T,
}

impl<A: TransitionAnimator<T>, T> TransitionAnimation<A, T> {
  pub fn update(&mut self, new: T, ctx: &mut UpdateCtx) -> &T {
    self.target = new;
    self
      .animator
      .update(&mut self.current, &self.target, todo!());
    &self.current
  }
}
