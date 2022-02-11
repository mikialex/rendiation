pub mod shader_builder;
pub use shader_builder::*;

pub mod operator;
pub use operator::*;

pub mod structor;
pub use structor::*;

pub mod swizzle;
pub use swizzle::*;

pub mod control;
pub use control::*;

use crate::*;

#[must_use]
pub fn consts<T>(v: T) -> Node<T>
where
  T: PrimitiveShaderGraphNodeType,
{
  v.into()
}
