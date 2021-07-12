mod examples;

#[macro_use]
mod lens;
pub use lens::*;

mod ability;
pub use ability::*;

mod structure;
pub use structure::*;

mod events;
pub use events::*;

mod layout;
pub use layout::*;

mod animation;
pub use animation::*;

mod rendering;
pub use rendering::*;

mod renderer;
pub use renderer::*;

mod components;
pub use components::*;

mod util;
pub use util::*;

pub trait Component<T> {
  fn event(&mut self, model: &mut T, event: &mut EventCtx) {}

  fn update(&mut self, model: &T, ctx: &mut UpdateCtx) {}
}

pub struct UpdateCtx {
  time_stamp: u64,
}

trait ComponentExt<T>: Component<T> + Sized {
  fn extend<A: ComponentAbility<T, Self>>(self, ability: A) -> Ability<T, Self, A> {
    Ability::new(self, ability)
  }
  fn lens<S, L: Lens<S, T>>(self, lens: L) -> LensWrap<S, T, L, Self> {
    LensWrap::new(self, lens)
  }
}

impl<X, T> ComponentExt<T> for X where X: Component<T> + Sized {}

pub struct UI<T> {
  root: Box<dyn Component<T>>,
}

impl<T> UI<T> {
  pub fn create(root: impl Component<T> + 'static) -> Self {
    Self {
      root: Box::new(root),
    }
  }
}
