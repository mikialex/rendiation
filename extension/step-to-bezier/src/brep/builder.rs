use fast_hash_collection::*;

use super::*;

/// Error message of a failed resolution. It is turned into a warning by whoever decides to
/// skip the entity.
pub(super) type R<T> = Result<T, String>;

/// Resolution state: the model under construction plus per STEP id caches (failures are cached
/// too, so a broken shared entity is reported once and every user sees the same cause).
pub(super) struct Builder<'a> {
  pub t: &'a Table,
  pub m: BrepModel,
  /// Radians per file plane angle unit.
  pub angle_factor: f64,
  pub faces: FastHashMap<u64, Option<FaceId>>,
  pub edges: FastHashMap<u64, R<EdgeId>>,
  pub vertices: FastHashMap<u64, R<VertexId>>,
  pub surfaces: FastHashMap<u64, R<SurfaceId>>,
  pub curves: FastHashMap<u64, R<CurveId>>,
  /// Keyed by STEP id and the bit pattern of the angular scale applied to the coordinates (see
  /// `geometry::pcurve_scale`).
  pub curves_2d: FastHashMap<(u64, [u64; 2]), R<Curve2Id>>,
  pub bodies: FastHashMap<u64, BodyId>,
  /// Geometry entities currently being converted, guards against reference cycles.
  pub in_progress: FastHashSet<u64>,
}

impl<'a> Builder<'a> {
  pub fn new(t: &'a Table) -> Self {
    Self {
      t,
      m: BrepModel::default(),
      angle_factor: 1.0,
      faces: Default::default(),
      edges: Default::default(),
      vertices: Default::default(),
      surfaces: Default::default(),
      curves: Default::default(),
      curves_2d: Default::default(),
      bodies: Default::default(),
      in_progress: Default::default(),
    }
  }

  pub fn build(mut self) -> BrepModel {
    self.resolve_units();
    self.angle_factor = self.m.plane_angle_unit_rad.unwrap_or(1.0);
    self.resolve_bodies();
    self.resolve_occurrences();
    self.m
  }

  pub fn warn(&mut self, message: impl Into<String>) {
    self.m.warnings.push(message.into());
  }

  /// Run a geometry conversion with a cycle guard on `id`.
  pub fn guarded<T>(&mut self, id: u64, f: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
    if !self.in_progress.insert(id) {
      return Err(format!("#{id} references itself"));
    }
    let r = f(self);
    self.in_progress.remove(&id);
    r
  }
}

/// Sorted keys of a step reader table, so that dense ids and warnings are deterministic.
#[allow(clippy::disallowed_types)]
pub(super) fn sorted_ids<T>(map: &std::collections::HashMap<u64, T>) -> Vec<u64> {
  let mut ids: Vec<_> = map.keys().copied().collect();
  ids.sort_unstable();
  ids
}
