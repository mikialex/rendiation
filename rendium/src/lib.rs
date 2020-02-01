pub mod renderer;
pub mod element;
pub mod component;
use component::*;

pub struct GUI<T: Component<T>> {
  state: T,
  root: ComponentInstance<T>
}