// pub trait Animator {}

pub struct WithAnimation<A, C> {
  animator: A,
  inner_current: C,
  inner_target: C,
}
