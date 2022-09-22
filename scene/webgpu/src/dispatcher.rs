use crate::{LightableSurfaceShadingDyn, RenderComponentAny};

pub trait DispatcherDyn {
  fn create_pass_gpu(
    &self,
    preferred_shading: Option<&'static dyn LightableSurfaceShadingDyn>,
  ) -> &dyn RenderComponentAny;
}

pub trait DispatcherDynSelf {}

impl<T> DispatcherDyn for T
where
  T: DispatcherDynSelf + RenderComponentAny,
{
  fn create_pass_gpu(
    &self,
    _: Option<&'static dyn LightableSurfaceShadingDyn>,
  ) -> &dyn RenderComponentAny {
    self
  }
}
