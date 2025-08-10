mod delta;
pub use delta::*;

mod previous_view;
pub use previous_view::*;

pub struct DualQuery<T, U> {
  pub view: T,
  pub delta: U,
}
