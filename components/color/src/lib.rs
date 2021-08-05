#![allow(clippy::many_single_char_names)]

pub mod hsl;
pub mod rgb;

pub use hsl::*;
pub use rgb::*;

pub struct WithAlpha<C, T> {
  pub color: C,
  pub a: T,
}
