#![feature(stmt_expr_attributes)]
#![feature(capture_disjoint_fields)]
#![feature(generic_associated_types)]
#![feature(associated_type_bounds)]
#![feature(min_type_alias_impl_trait)]
#![allow(incomplete_features)]

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

mod fonts;
pub use fonts::*;

mod animation;
pub use animation::*;

mod rendering;
pub use rendering::*;

mod renderer;
pub use renderer::*;

mod components;
pub use components::*;

mod memo;
pub use memo::*;

mod utils;
pub use utils::*;

mod app;
pub use app::*;

pub trait Component<T, S: System = DefaultSystem> {
  fn event(&mut self, _model: &mut T, _vent: &mut S::EventCtx<'_>) {}

  fn update(&mut self, _model: &T, _ctx: &mut S::UpdateCtx<'_>) {}
}

pub trait System {
  type EventCtx<'a>;
  type UpdateCtx<'a>;
}

pub struct DefaultSystem {}

impl System for DefaultSystem {
  type EventCtx<'a> = EventCtx<'a>;
  type UpdateCtx<'a> = UpdateCtx;
}

pub struct UpdateCtx {
  pub time_stamp: u64,
  layout_changed: bool,
}

impl UpdateCtx {
  pub fn request_layout(&mut self) {
    self.layout_changed = true;
  }
}

pub trait ComponentExt<T>: Component<T> + Sized {
  fn extend<A: ComponentAbility<T, Self>>(self, ability: A) -> Ability<T, Self, A> {
    Ability::new(self, ability)
  }
  fn lens<S, L: Lens<S, T>>(self, lens: L) -> LensWrap<S, T, L, Self> {
    LensWrap::new(self, lens)
  }
}

impl<X, T> ComponentExt<T> for X where X: Component<T> + Sized {}

pub trait UIComponent<T>: Component<T> + Presentable + LayoutAble + 'static {}
impl<X, T> UIComponent<T> for X where X: Component<T> + Presentable + LayoutAble + 'static {}
