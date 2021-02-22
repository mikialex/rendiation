mod source;
mod transformer;
use std::hash::Hash;

pub use source::*;
pub use transformer::*;

use crate::RenderGraphBackend;

pub enum ContentKey<T: RenderGraphBackend> {
  Source(T::ContentSourceKey),
  Inner(T::ContentMiddleKey),
}

impl<T: RenderGraphBackend> PartialEq for ContentKey<T> {
  fn eq(&self, other: &Self) -> bool {
    self == other
  }
}
impl<T: RenderGraphBackend> Eq for ContentKey<T> {}
impl<T: RenderGraphBackend> Copy for ContentKey<T> {}
impl<T: RenderGraphBackend> Clone for ContentKey<T> {
  fn clone(&self) -> Self {
    match self {
      ContentKey::Source(s) => ContentKey::Source(*s),
      ContentKey::Inner(i) => ContentKey::Inner(*i),
    }
  }
}
impl<T: RenderGraphBackend> Hash for ContentKey<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    match self {
      ContentKey::Source(s) => s.hash(state),
      ContentKey::Inner(i) => i.hash(state),
    }
  }
}
