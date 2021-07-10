use std::marker::PhantomData;

mod example;

mod lens;
pub use lens::*;

mod structure;
pub use structure::*;

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

// pub trait Passthrough<T> {
//   fn visit(&self, f: impl FnMut(&dyn Component<T>)) {}
//   fn mutate(&mut self, f: impl FnMut(&mut dyn Component<T>)) {}
// }

struct Ability<T, C, A>
where
  C: Component<T>,
  A: ComponentAbility<T, C>,
{
  inner: C,
  ability: A,
  phantom: PhantomData<T>,
}

pub trait ComponentAbility<T, C: Component<T>> {
  fn update(&mut self, model: &T, inner: &mut C) {
    inner.update(model);
  }
  fn event(&mut self, model: &mut T, event: &winit::event::Event<()>, inner: &mut C) {
    inner.event(model, event);
  }
}

impl<T, C, A> Component<T> for Ability<T, C, A>
where
  C: Component<T>,
  A: ComponentAbility<T, C>,
{
  fn update(&mut self, model: &T) {
    self.ability.update(model, &mut self.inner);
  }
  fn event(&mut self, model: &mut T, event: &winit::event::Event<()>) {
    self.ability.event(model, event, &mut self.inner);
  }
}

pub struct ClickArea<T, C> {
  inner: C,
  phantom: PhantomData<T>,
}

impl<T, C: Component<T>> ComponentExt<T> for C {}

trait ComponentExt<T>: Component<T> + Sized {
  fn sized(self, width: f32, height: f32) -> Container<T, Self> {
    Container {
      width,
      height,
      inner: self,
      phantom: PhantomData,
    }
  }
  fn on(self, func: impl Fn(&mut T) + 'static) -> Ability<T, Self, EventHandler<T>> {
    todo!()
  }
}

pub struct Container<T, C> {
  width: f32,
  height: f32,
  inner: C,
  phantom: PhantomData<T>,
}

impl<T, C: Component<T>> Component<T> for Container<T, C> {}

struct EventHandler<T> {
  handler: Box<dyn Fn(&mut T)>,
}

pub trait Test {}

impl<T, C: Component<T>> ComponentAbility<T, C> for EventHandler<T> {
  fn event(&mut self, model: &mut T, event: &winit::event::Event<()>, inner: &mut C) {}
}
