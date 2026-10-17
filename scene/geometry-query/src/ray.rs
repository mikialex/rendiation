use crate::*;

/// The ray parameter where the ray enters the box, zero if the ray origin is inside the box, none
/// if the ray misses the box. It is a lower bound of the t of any hit inside the box.
pub fn ray_aabb_entry_t<T: Scalar>(ray: &Ray3<T>, bbox: &Box3<T>) -> Option<T> {
  if bbox.is_empty() {
    return None;
  }
  let origin = [ray.origin.x, ray.origin.y, ray.origin.z];
  let direction = [ray.direction.x, ray.direction.y, ray.direction.z];
  let min = [bbox.min.x, bbox.min.y, bbox.min.z];
  let max = [bbox.max.x, bbox.max.y, bbox.max.z];

  let mut t_enter = T::zero();
  let mut t_exit = T::infinity();
  for axis in 0..3 {
    let (o, d) = (origin[axis], direction[axis]);
    if d == T::zero() {
      if o < min[axis] || o > max[axis] {
        return None;
      }
      continue;
    }
    let inv = T::one() / d;
    let mut t0 = (min[axis] - o) * inv;
    let mut t1 = (max[axis] - o) * inv;
    if t0 > t1 {
      std::mem::swap(&mut t0, &mut t1);
    }
    t_enter = t_enter.max(t0);
    t_exit = t_exit.min(t1);
    if t_enter > t_exit {
      return None;
    }
  }
  Some(t_enter)
}

/// A ray hit in world space.
#[derive(Clone, Copy, Debug)]
pub struct SceneRayHit {
  pub model: EntityHandle<SceneModelEntity>,
  /// the instance index if the model is an instanced model
  pub instance: Option<u32>,
  /// the primitive index inside the (source) model, the meaning of primitive is defined by the
  /// local model picker, for example quad index for text and cell mesh
  pub primitive: u32,
  /// the world space ray parameter of the hit position's projection on the ray.
  ///
  /// for a hit with tolerance (line, point), the position is not on the ray, so the projection
  /// rather than the euclidean distance is used. this guarantees t is not less than the entry
  /// parameter of any tolerance enlarged bounding that contains the position, which bounding based
  /// pruning relies on.
  pub t: f64,
  pub position: Vec3<f64>,
}

/// The consumer of ray hits.
pub trait RayHitVisitor {
  /// hits whose t is larger than this can be skipped by the producer.
  ///
  /// the value may shrink after each visit, so producers should read it again instead of caching.
  fn t_max(&self) -> f64;
  /// return break to stop the whole query
  fn visit(&mut self, hit: SceneRayHit) -> ControlFlow<()>;
}

impl<T: RayHitVisitor + ?Sized> RayHitVisitor for &mut T {
  fn t_max(&self) -> f64 {
    (**self).t_max()
  }
  fn visit(&mut self, hit: SceneRayHit) -> ControlFlow<()> {
    (**self).visit(hit)
  }
}

#[derive(Default)]
pub struct NearestRayHit {
  pub nearest: Option<SceneRayHit>,
}

impl RayHitVisitor for NearestRayHit {
  fn t_max(&self) -> f64 {
    self.nearest.map(|h| h.t).unwrap_or(f64::INFINITY)
  }
  fn visit(&mut self, hit: SceneRayHit) -> ControlFlow<()> {
    if hit.t < self.t_max() {
      self.nearest = Some(hit);
    }
    ControlFlow::Continue(())
  }
}

/// collect all hits, the order is not defined
#[derive(Default)]
pub struct AllRayHits {
  pub hits: Vec<SceneRayHit>,
}

impl AllRayHits {
  pub fn into_sorted(mut self) -> Vec<SceneRayHit> {
    self.hits.sort_by(|a, b| a.t.total_cmp(&b.t));
    self.hits
  }
}

impl RayHitVisitor for AllRayHits {
  fn t_max(&self) -> f64 {
    f64::INFINITY
  }
  fn visit(&mut self, hit: SceneRayHit) -> ControlFlow<()> {
    self.hits.push(hit);
    ControlFlow::Continue(())
  }
}

/// stop at the first hit, the hit is not guaranteed to be the nearest one.
/// useful for occlusion or hover test.
#[derive(Default)]
pub struct AnyRayHit {
  pub hit: Option<SceneRayHit>,
}

impl RayHitVisitor for AnyRayHit {
  fn t_max(&self) -> f64 {
    f64::INFINITY
  }
  fn visit(&mut self, hit: SceneRayHit) -> ControlFlow<()> {
    self.hit = Some(hit);
    ControlFlow::Break(())
  }
}

/// keep the k nearest hits, sorted by t ascending
pub struct KNearestRayHits {
  k: usize,
  hits: Vec<SceneRayHit>,
}

impl KNearestRayHits {
  pub fn new(k: usize) -> Self {
    Self {
      k,
      hits: Vec::with_capacity(k),
    }
  }
  pub fn hits(&self) -> &[SceneRayHit] {
    &self.hits
  }
}

impl RayHitVisitor for KNearestRayHits {
  fn t_max(&self) -> f64 {
    if self.k == 0 {
      f64::NEG_INFINITY
    } else if self.hits.len() == self.k {
      self.hits[self.k - 1].t
    } else {
      f64::INFINITY
    }
  }
  fn visit(&mut self, hit: SceneRayHit) -> ControlFlow<()> {
    if hit.t < self.t_max() {
      let position = self.hits.partition_point(|h| h.t <= hit.t);
      self.hits.insert(position, hit);
      self.hits.truncate(self.k);
    }
    ControlFlow::Continue(())
  }
}

/// only forward hits that pass the filter. `t_max` is forwarded from the inner visitor, so
/// rejected hits never shrink the pruning bound.
pub struct FilteredRayHits<V, F> {
  pub inner: V,
  /// keep the hit if return true
  pub filter: F,
}

impl<V, F> RayHitVisitor for FilteredRayHits<V, F>
where
  V: RayHitVisitor,
  F: FnMut(&SceneRayHit) -> bool,
{
  fn t_max(&self) -> f64 {
    self.inner.t_max()
  }
  fn visit(&mut self, hit: SceneRayHit) -> ControlFlow<()> {
    if (self.filter)(&hit) {
      self.inner.visit(hit)
    } else {
      ControlFlow::Continue(())
    }
  }
}

/// Used by instanced picker. The source model's hits are rewritten to belong to the instanced
/// model and tagged with the instance index, the primitive index is kept, so the filter sees the
/// instanced model and the sub primitive information is not lost.
pub struct RayHitInstanceTag<'a> {
  pub inner: &'a mut dyn RayHitVisitor,
  pub model: EntityHandle<SceneModelEntity>,
  pub instance: u32,
}

impl RayHitVisitor for RayHitInstanceTag<'_> {
  fn t_max(&self) -> f64 {
    self.inner.t_max()
  }
  fn visit(&mut self, mut hit: SceneRayHit) -> ControlFlow<()> {
    hit.model = self.model;
    hit.instance = Some(self.instance);
    self.inner.visit(hit)
  }
}

#[derive(Clone, Copy)]
pub struct SceneModelRayQuery<'a> {
  pub idx: EntityHandle<SceneModelEntity>,
  /// if the override_world_mat used, the internal node matrix is ignored
  pub override_world_mat: Option<&'a Mat4<f64>>,
  pub ctx: &'a SceneRayQuery,
  /// ignore selectable or visible states
  pub ignore_pre_check: bool,
}

#[derive(Clone, Copy)]
pub struct LocalRayQuery<'a> {
  pub idx: EntityHandle<SceneModelEntity>,
  pub local_ray: Ray3<f32>,
  pub local_tolerance: f32,
  pub extra_screen_space_tolerance: f32,
  pub world_mat: &'a Mat4<f64>,
  pub camera_ctx: &'a CameraQueryCtx,
}

/// A ray hit in model local space.
#[derive(Clone, Copy)]
pub struct LocalRayHit {
  pub primitive: u32,
  /// the local space ray parameter of the hit position's projection on the local ray
  pub t: f32,
  pub position: Vec3<f32>,
}

/// The consumer of local ray hits, see [RayHitVisitor].
pub trait LocalRayHitVisitor {
  fn t_max(&self) -> f32;
  fn visit(&mut self, hit: LocalRayHit) -> ControlFlow<()>;
}

/// Used by the base scene model picker to connect local model pickers with the world space
/// visitor. It converts local hits into world hits, and the world `t_max` into local `t_max`.
pub struct LocalToWorldRayHitAdapter<'a> {
  inner: &'a mut dyn RayHitVisitor,
  model: EntityHandle<SceneModelEntity>,
  world_mat: Mat4<f64>,
  world_ray: Ray3<f64>,
  /// world space length of a unit step along the local ray direction.
  /// the mapping is affine so the local and world ray parameter differ by this constant factor.
  local_to_world_t_scale: f64,
}

impl<'a> LocalToWorldRayHitAdapter<'a> {
  pub fn new(
    inner: &'a mut dyn RayHitVisitor,
    model: EntityHandle<SceneModelEntity>,
    world_mat: Mat4<f64>,
    world_ray: Ray3<f64>,
    local_ray: &Ray3<f32>,
  ) -> Self {
    let local_direction = (*local_ray.direction).into_f64();
    let local_to_world_t_scale = (world_mat.to_mat3() * local_direction).length();
    Self {
      inner,
      model,
      world_mat,
      world_ray,
      local_to_world_t_scale,
    }
  }
}

impl LocalRayHitVisitor for LocalToWorldRayHitAdapter<'_> {
  fn t_max(&self) -> f32 {
    (self.inner.t_max() / self.local_to_world_t_scale) as f32
  }
  fn visit(&mut self, hit: LocalRayHit) -> ControlFlow<()> {
    let position = hit.position.into_f64().apply_matrix_into(self.world_mat);
    let t = (position - self.world_ray.origin).dot(*self.world_ray.direction);
    self.inner.visit(SceneRayHit {
      model: self.model,
      instance: None,
      primitive: hit.primitive,
      t,
      position,
    })
  }
}
