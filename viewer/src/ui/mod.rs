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

mod rendering;
pub use rendering::*;

mod renderer;
pub use renderer::*;

mod components;
pub use components::*;

mod util;
pub use util::*;

pub trait Component<T> {
  fn event(&mut self, model: &mut T, event: &winit::event::Event<()>) {}

  fn update(&mut self, model: &T) {}
}

pub trait EventHandler<T> {
  type Event;
  fn event(&mut self, model: &mut T, event: &winit::event::Event<()>) -> Option<Self::Event> {
    None
  }
}

trait ComponentExt<T>: Component<T> + Sized {
  fn extend<A: ComponentAbility<T, Self>>(self, ability: A) -> Ability<T, Self, A> {
    Ability::new(self, ability)
  }
  fn lens<S, L: Lens<S, T>>(self, lens: L) -> LensWrap<S, T, L, Self> {
    LensWrap::new(self, lens)
  }
  // fn sized(self, width: f32, height: f32) -> Container<T, Self> {
  //   Container {
  //     width,
  //     height,
  //     inner: self,
  //     phantom: PhantomData,
  //   }
  // }
  // fn on(self, func: impl Fn(&mut T) + 'static) -> Ability<T, Self, EventHandler<T>> {
  //   todo!()
  // }
}

impl<X, T> ComponentExt<T> for X where X: Component<T> + Sized {}

pub struct UI<T> {
  root: Box<dyn Component<T>>,
}
