use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectTestPolicy {
  Intersect,
  Contains,
}

/// A contiguous range of primitives sharing the same test result.
///
/// The per primitive test depends on the policy of the query: for intersect, a primitive passes if
/// it intersects the frustum; for contains, a primitive passes if it is fully inside the frustum.
///
/// Producers emit a single primitive event when testing primitive one by one, and a whole range
/// pass event when the model bounding is fully inside the frustum. When the bounding is fully
/// outside, producers emit nothing, every standard visitor gives the correct result without any
/// event.
pub struct FrustumPrimitiveEvent {
  /// filled by instanced picker, none for normal model
  pub instance: Option<u32>,
  pub primitives: Range<u32>,
  pub pass: bool,
}

impl FrustumPrimitiveEvent {
  pub fn single(primitive: u32, pass: bool) -> Self {
    Self {
      instance: None,
      primitives: primitive..primitive + 1,
      pass,
    }
  }
  pub fn whole(primitive_count: u32, pass: bool) -> Self {
    Self {
      instance: None,
      primitives: 0..primitive_count,
      pass,
    }
  }
}

/// The consumer of frustum primitive events. A visitor is created per scene model, the model level
/// verdict is a reduction over the events of that model.
pub trait FrustumPrimitiveVisitor {
  /// return break to stop testing the current model
  fn visit(&mut self, event: FrustumPrimitiveEvent) -> ControlFlow<()>;
}

impl<T: FrustumPrimitiveVisitor + ?Sized> FrustumPrimitiveVisitor for &mut T {
  fn visit(&mut self, event: FrustumPrimitiveEvent) -> ControlFlow<()> {
    (**self).visit(event)
  }
}

/// model level, intersect policy: selected if any primitive passes
#[derive(Default)]
pub struct ModelIntersectVisitor {
  pub hit: bool,
}

impl FrustumPrimitiveVisitor for ModelIntersectVisitor {
  fn visit(&mut self, event: FrustumPrimitiveEvent) -> ControlFlow<()> {
    if event.pass && !event.primitives.is_empty() {
      self.hit = true;
      return ControlFlow::Break(());
    }
    ControlFlow::Continue(())
  }
}

/// model level, contains policy: selected if all primitives pass.
///
/// the fail event is only meaningful here, it allows to stop at the first primitive that is not
/// fully inside.
#[derive(Default)]
pub struct ModelContainsVisitor {
  any_pass: bool,
  any_fail: bool,
}

impl ModelContainsVisitor {
  /// an empty model is not contained
  pub fn contained(&self) -> bool {
    self.any_pass && !self.any_fail
  }
}

impl FrustumPrimitiveVisitor for ModelContainsVisitor {
  fn visit(&mut self, event: FrustumPrimitiveEvent) -> ControlFlow<()> {
    if event.primitives.is_empty() {
      return ControlFlow::Continue(());
    }
    if event.pass {
      self.any_pass = true;
      ControlFlow::Continue(())
    } else {
      self.any_fail = true;
      ControlFlow::Break(())
    }
  }
}

/// model level visitor chosen by policy
pub enum ModelFrustumVisitor {
  Intersect(ModelIntersectVisitor),
  Contains(ModelContainsVisitor),
}

impl ModelFrustumVisitor {
  pub fn new(policy: ObjectTestPolicy) -> Self {
    match policy {
      ObjectTestPolicy::Intersect => Self::Intersect(Default::default()),
      ObjectTestPolicy::Contains => Self::Contains(Default::default()),
    }
  }
  pub fn selected(&self) -> bool {
    match self {
      Self::Intersect(v) => v.hit,
      Self::Contains(v) => v.contained(),
    }
  }
}

impl FrustumPrimitiveVisitor for ModelFrustumVisitor {
  fn visit(&mut self, event: FrustumPrimitiveEvent) -> ControlFlow<()> {
    match self {
      Self::Intersect(v) => v.visit(event),
      Self::Contains(v) => v.visit(event),
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubPrimitiveId {
  pub instance: Option<u32>,
  pub primitive: u32,
}

/// sub primitive level, collect all passed primitives, works for both policy
pub struct SubPrimitiveVisitor<'a> {
  pub results: &'a mut Vec<SubPrimitiveId>,
}

impl FrustumPrimitiveVisitor for SubPrimitiveVisitor<'_> {
  fn visit(&mut self, event: FrustumPrimitiveEvent) -> ControlFlow<()> {
    if event.pass {
      let instance = event.instance;
      self
        .results
        .extend(event.primitives.map(|primitive| SubPrimitiveId {
          instance,
          primitive,
        }));
    }
    ControlFlow::Continue(())
  }
}

/// Instance level selection for instanced models, sub primitive level for normal models.
///
/// For an instanced model, each instance is reduced like a model: selected if any primitive passes
/// for intersect, or all primitives pass for contains. For a normal model, passed primitives are
/// collected. Events of one instance are expected to be contiguous, which the instanced picker
/// guarantees by testing instance by instance.
pub struct InstanceOrSubPrimitiveVisitor {
  policy: ObjectTestPolicy,
  /// (instance, any_pass, any_fail)
  instances: Vec<(u32, bool, bool)>,
  primitives: Vec<u32>,
}

impl InstanceOrSubPrimitiveVisitor {
  pub fn new(policy: ObjectTestPolicy) -> Self {
    Self {
      policy,
      instances: Vec::new(),
      primitives: Vec::new(),
    }
  }

  /// selected instance indices for instanced model, or passed primitive indices for normal model
  pub fn finish(self, results: &mut Vec<u32>) {
    let policy = self.policy;
    results.extend(
      self
        .instances
        .into_iter()
        .filter(|&(_, any_pass, any_fail)| match policy {
          ObjectTestPolicy::Intersect => any_pass,
          ObjectTestPolicy::Contains => any_pass && !any_fail,
        })
        .map(|(instance, _, _)| instance),
    );
    results.extend(self.primitives);
  }
}

impl FrustumPrimitiveVisitor for InstanceOrSubPrimitiveVisitor {
  fn visit(&mut self, event: FrustumPrimitiveEvent) -> ControlFlow<()> {
    if event.primitives.is_empty() {
      return ControlFlow::Continue(());
    }
    if let Some(instance) = event.instance {
      let state = match self.instances.last_mut() {
        Some(state) if state.0 == instance => state,
        _ => {
          self.instances.push((instance, false, false));
          self.instances.last_mut().unwrap()
        }
      };
      if event.pass {
        state.1 = true;
      } else {
        state.2 = true;
      }
    } else if event.pass {
      self.primitives.extend(event.primitives);
    }
    ControlFlow::Continue(())
  }
}

/// Used by instanced picker, tag the events of the source model with the instance index.
///
/// with this, the instanced picker does not need its own any or all logic: model level intersect
/// becomes "any primitive of any instance passes", contains becomes "all primitives of all
/// instances pass", and sub primitive results keep both instance and primitive index.
pub struct FrustumInstanceTag<'a> {
  pub inner: &'a mut dyn FrustumPrimitiveVisitor,
  pub instance: u32,
}

impl FrustumPrimitiveVisitor for FrustumInstanceTag<'_> {
  fn visit(&mut self, mut event: FrustumPrimitiveEvent) -> ControlFlow<()> {
    event.instance = Some(self.instance);
    self.inner.visit(event)
  }
}

#[derive(Clone, Copy)]
pub struct SceneModelFrustumQuery<'a> {
  pub idx: EntityHandle<SceneModelEntity>,
  /// if the override_world_mat used, the internal node matrix is ignored
  pub override_world_mat: Option<&'a Mat4<f64>>,
  pub frustum: &'a SceneFrustumQuery,
  pub policy: ObjectTestPolicy,
  /// the TLAS already proved the model bounding is fully inside the frustum, the producer can
  /// emit a whole range pass event without any further test
  pub known_inside: bool,
  /// ignore selectable or visible states
  pub ignore_pre_check: bool,
}

#[derive(Clone, Copy)]
pub struct LocalFrustumQuery<'a> {
  pub idx: EntityHandle<SceneModelEntity>,
  pub local_frustum: &'a Frustum<f32>,
  pub helper: Option<&'a FrustumIntersectionTestHelper<f32>>,
  pub policy: ObjectTestPolicy,
  pub extra_screen_space_tolerance: f32,
  pub world_mat: &'a Mat4<f64>,
  pub camera_ctx: &'a CameraQueryCtx,
}
