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
///   transform the query into local space, resolve the model by the [LocalModelPickerResolver] and
///   connect it with [LocalToWorldRayHitAdapter] for ray.
///
/// A picker that does not handle the model, or fails to access the required data, simply produces
/// no hits and returns continue, the data error is logged. Returning break means the visitor
/// requested to stop.
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

pub type ResolvedLocalModelPickerBox<'a> =
  Box<dyn ResolvedLocalModelPicker + 'a, &'a dyn Allocator>;

/// The model is selected by a resolver, but the data it requires is missing or invalid.
#[derive(Debug, thiserror::Error)]
#[error("invalid local model pick data: {0}")]
pub struct LocalModelPickDataError(pub &'static str);

/// Resolve the local space picker of a model for one kind of model data (attribute mesh, wide
/// line, text, ...).
///
/// A list of resolvers is also a resolver, it selects the first one responsible for the model.
pub trait LocalModelPickerResolver {
  /// Return None if this resolver is not responsible for the model. The responsibility is decided
  /// by the kind of the model data only, so the resolvers are mutually exclusive. Once selected,
  /// missing or invalid data is reported as error instead of falling back to other resolvers.
  ///
  /// The resolve is done once per model query, all the query methods of the model query are
  /// called on the same resolved picker.
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedLocalModelPickerBox<'a>, LocalModelPickDataError>>;
}

impl LocalModelPickerResolver for [Box<dyn LocalModelPickerResolver>] {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedLocalModelPickerBox<'a>, LocalModelPickDataError>> {
    self
      .iter()
      .find_map(|resolver| resolver.resolve(idx, alloc))
  }
}

impl LocalModelPickerResolver for Vec<Box<dyn LocalModelPickerResolver>> {
  fn resolve<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    alloc: &'a dyn Allocator,
  ) -> Option<Result<ResolvedLocalModelPickerBox<'a>, LocalModelPickDataError>> {
    self.as_slice().resolve(idx, alloc)
  }
}

/// The local space picker resolved for one model.
///
/// The resolve should only access the data required for the bounding test, heavy data like the
/// mesh view should be accessed in the query methods, because the model may be culled before.
pub trait ResolvedLocalModelPicker {
  /// the tolerance of line or point like primitives. it is used to enlarge the bounding before
  /// the bounding test and to compute the local tolerance passed to the local query.
  fn tolerance(&self) -> Option<IntersectTolerance>;

  /// used by the frustum bounding shortcut to emit a whole range event
  fn primitive_count(&self) -> Result<u32, LocalModelPickDataError>;

  /// push hits whose local t is not larger than `v.t_max()`, the producer may use the bound to skip
  /// primitives
  fn ray_query_local(
    &self,
    req: LocalRayQuery,
    v: &mut dyn LocalRayHitVisitor,
  ) -> Result<ControlFlow<()>, LocalModelPickDataError>;

  /// test primitives one by one with the policy of the request and emit
  /// [FrustumPrimitiveEvent::single], stop when the visitor breaks
  fn frustum_query_local(
    &self,
    req: LocalFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
  ) -> Result<ControlFlow<()>, LocalModelPickDataError>;
}
