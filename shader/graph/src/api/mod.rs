mod shader_builder;
pub use shader_builder::*;

mod operator;
pub use operator::*;

mod structor;
pub use structor::*;

mod swizzle;
pub use swizzle::*;

mod math;
pub use math::*;

mod control;
pub use control::*;

use crate::*;

#[must_use]
pub fn consts<T>(v: T) -> Node<T>
where
  T: PrimitiveShaderGraphNodeType,
{
  v.into()
}
