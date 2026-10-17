use crate::*;

/// Scene model level picker.
///
/// Implementations are usually decorators around a base implementation:
///
/// - view dependent transform decorator: resolve the view dependent matrix and forward it as
///   `override_world_mat`.
/// - instanced decorator: if the model is an instanced model, query the source model once per
///   instance with `override_world_mat`, wrap the visitor with [RayHitInstanceTag] or
///   [FrustumInstanceTag]; otherwise forward to the inner picker.
/// - base implementation: pre check (selectable, visible), resolve the world matrix and bounding,
///   do the bounding test (prune by `t_max` for ray, inside or outside shortcut for frustum),
///   transform the query into local space, select the [LocalModelPicker] and connect it with
///   [LocalToWorldRayHitAdapter] for ray.
///
/// A picker that does not handle the model, or fails to access the required data, simply produces
/// no hits and returns continue. Returning break means the visitor requested to stop.
pub trait SceneModelPicker {
  fn ray_query(&self, req: SceneModelRayQuery, v: &mut dyn RayHitVisitor) -> ControlFlow<()>;

  fn frustum_query(
    &self,
    req: SceneModelFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
  ) -> ControlFlow<()>;
}

impl<T: SceneModelPicker + ?Sized> SceneModelPicker for Box<T> {
  fn ray_query(&self, req: SceneModelRayQuery, v: &mut dyn RayHitVisitor) -> ControlFlow<()> {
    (**self).ray_query(req, v)
  }

  fn frustum_query(
    &self,
    req: SceneModelFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
  ) -> ControlFlow<()> {
    (**self).frustum_query(req, v)
  }
}

/// Model local space picker for one kind of model data (attribute mesh, wide line, text, ...).
///
/// The query methods are only called for models this picker is selected for, see
/// [LocalModelPickerSelector], so they do not need to express "not applicable" in the return
/// value.
pub trait LocalModelPicker {
  /// return true if this picker is responsible for the model
  fn is_applicable(&self, idx: EntityHandle<SceneModelEntity>) -> bool;

  /// the tolerance of line or point like primitives. it is used to enlarge the bounding before
  /// the bounding test and to compute the local tolerance passed to the local query.
  fn tolerance(&self, idx: EntityHandle<SceneModelEntity>) -> Option<IntersectTolerance>;

  /// used by the frustum bounding shortcut to emit a whole range event
  fn primitive_count(&self, idx: EntityHandle<SceneModelEntity>) -> u32;

  /// push hits whose local t is not larger than `v.t_max()`, the producer may use the bound to skip
  /// primitives
  fn ray_query_local(&self, req: LocalRayQuery, v: &mut dyn LocalRayHitVisitor) -> ControlFlow<()>;

  /// test primitives one by one with the policy of the request and emit
  /// [FrustumPrimitiveEvent::single], stop when the visitor breaks
  fn frustum_query_local(
    &self,
    req: LocalFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
  ) -> ControlFlow<()>;
}

/// Select the local picker responsible for a model. The selection is done once per model query,
/// instead of asking every local picker in turn for every method.
pub trait LocalModelPickerSelector {
  fn select(&self, idx: EntityHandle<SceneModelEntity>) -> Option<&dyn LocalModelPicker>;
}

impl LocalModelPickerSelector for [Box<dyn LocalModelPicker>] {
  fn select(&self, idx: EntityHandle<SceneModelEntity>) -> Option<&dyn LocalModelPicker> {
    self
      .iter()
      .find(|picker| picker.is_applicable(idx))
      .map(|picker| picker.as_ref())
  }
}

impl LocalModelPickerSelector for Vec<Box<dyn LocalModelPicker>> {
  fn select(&self, idx: EntityHandle<SceneModelEntity>) -> Option<&dyn LocalModelPicker> {
    self.as_slice().select(idx)
  }
}
