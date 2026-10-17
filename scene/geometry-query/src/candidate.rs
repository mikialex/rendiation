use crate::*;

#[derive(Clone, Copy)]
pub struct RayCandidate {
  pub model: EntityHandle<SceneModelEntity>,
  /// a lower bound of the t of any hit the model may produce, typically the entry parameter of
  /// the tolerance enlarged bounding. zero if unknown.
  pub t_enter: f64,
}

#[derive(Clone, Copy)]
pub struct FrustumCandidate {
  pub model: EntityHandle<SceneModelEntity>,
  /// the model bounding is known to be fully inside the frustum
  pub known_inside: bool,
}

/// The TLAS abstraction, enumerates candidate scene models of a scene.
pub trait SceneModelCandidateProvider {
  /// visit ray candidates, preferably in ascending `t_enter` order (near child first traversal),
  /// so that the bound shrinks early.
  ///
  /// `t_max` is the initial pruning bound, the callback returns the updated bound after each
  /// candidate. nodes or candidates whose entry parameter is larger than the current bound should
  /// be skipped. return break if the callback breaks.
  fn traverse_ray(
    &self,
    scene: EntityHandle<SceneEntity>,
    query: &SceneRayQuery,
    t_max: f64,
    f: &mut dyn FnMut(RayCandidate) -> ControlFlow<(), f64>,
  ) -> ControlFlow<()>;

  /// visit frustum candidates. return break if the callback breaks.
  fn traverse_frustum(
    &self,
    scene: EntityHandle<SceneEntity>,
    query: &SceneFrustumQuery,
    f: &mut dyn FnMut(FrustumCandidate) -> ControlFlow<()>,
  ) -> ControlFlow<()>;
}

/// An explicit model list, for example the intersection group of gui 3d. The scene is ignored and
/// no pruning is done at this level.
pub struct ModelListCandidates<'a>(pub &'a [EntityHandle<SceneModelEntity>]);

impl SceneModelCandidateProvider for ModelListCandidates<'_> {
  fn traverse_ray(
    &self,
    _scene: EntityHandle<SceneEntity>,
    _query: &SceneRayQuery,
    _t_max: f64,
    f: &mut dyn FnMut(RayCandidate) -> ControlFlow<(), f64>,
  ) -> ControlFlow<()> {
    for &model in self.0 {
      f(RayCandidate { model, t_enter: 0. })?;
    }
    ControlFlow::Continue(())
  }

  fn traverse_frustum(
    &self,
    _scene: EntityHandle<SceneEntity>,
    _query: &SceneFrustumQuery,
    f: &mut dyn FnMut(FrustumCandidate) -> ControlFlow<()>,
  ) -> ControlFlow<()> {
    for &model in self.0 {
      f(FrustumCandidate {
        model,
        known_inside: false,
      })?;
    }
    ControlFlow::Continue(())
  }
}
