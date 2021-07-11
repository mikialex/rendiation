use std::marker::PhantomData;

mod example;

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

pub trait Component<T> {
  fn event(&mut self, model: &mut T, event: &winit::event::Event<()>) {}

  fn update(&mut self, model: &T) {}
}

pub trait EventHandler<T, E> {
  type Event;
  fn event(&mut self, model: &mut T, event: &E) -> Option<Self::Event> {
    None
  }
}

trait ComponentExt<T>: Component<T> + Sized {
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
