mod attribute;
pub use attribute::*;

use crate::*;

pub trait GLESModelShapeRenderImpl {
  /// Return None if this impl is not responsible for the model. The responsibility is decided by
  /// the kind of the shape only, so the impls are mutually exclusive. Once selected, missing or
  /// invalid data is reported as error.
  fn make_component(
    &self,
    idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Result<(Box<dyn RenderComponent + '_>, DrawCommand), SceneModelRenderDataError>>;
}

impl GLESModelShapeRenderImpl for Vec<Box<dyn GLESModelShapeRenderImpl>> {
  fn make_component(
    &self,
    idx: EntityHandle<StandardModelEntity>,
  ) -> Option<Result<(Box<dyn RenderComponent + '_>, DrawCommand), SceneModelRenderDataError>> {
    self
      .iter()
      .find_map(|provider| provider.make_component(idx))
  }
}
