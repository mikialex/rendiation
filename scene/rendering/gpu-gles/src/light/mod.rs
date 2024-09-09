mod directional;
pub use directional::*;
mod point;
pub use point::*;
mod spot;
pub use spot::*;

use crate::*;

pub struct MultiUpdateContainerImplAbstractBindingSource<T: 'static>(
  pub LockReadGuardHolder<MultiUpdateContainer<T>>,
);
impl<T: AbstractBindingSource + 'static> AbstractBindingSource
  for MultiUpdateContainerImplAbstractBindingSource<T>
{
  type ShaderBindResult = T::ShaderBindResult;

  fn bind_pass(&self, ctx: &mut BindingBuilder) {
    self.0.bind_pass(ctx);
  }

  fn bind_shader(&self, ctx: &mut ShaderBindGroupBuilder) -> Self::ShaderBindResult {
    self.0.bind_shader(ctx)
  }
}
