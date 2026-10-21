//! Checks of a tessellation result: dense sampled screen space error, edge consistency
//! between neighboring leaves, and the triangle count of a per patch uniform tessellation for
//! comparison.

use fast_hash_collection::FastHashMap;
use parametric_normalized_model::*;
use rendiation_algebra::*;

use crate::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct ErrorReport {
  pub samples: usize,
  /// Max of `|T - S| · pixel_scale(S)` over all samples in front of the near plane.
  pub max_pixel_error: f64,
  pub samples_over_threshold: usize,
}

/// Samples every leaf triangle on a barycentric grid of `resolution` and compares the
/// interpolated position with the surface at the interpolated parameter.
pub fn measure_error(t: &mut Tessellator, leaves: &[Leaf], resolution: u32) -> ErrorReport {
  let meshes: Vec<_> = leaves.iter().map(|leaf| t.leaf_mesh(leaf)).collect();
  measure_mesh_error(t, leaves, &meshes, resolution)
}

/// `measure_error` of leaf meshes from another source (for example read back from the GPU),
/// `meshes[i]` is the grid of `leaves[i]`, its surface is the item surface of `t`.
pub fn measure_mesh_error(
  t: &Tessellator,
  leaves: &[Leaf],
  meshes: &[LeafMesh],
  resolution: u32,
) -> ErrorReport {
  let mut report = ErrorReport::default();
  let threshold = t.config().pixel_error;
  let r = resolution as f64;
  for (leaf, mesh) in leaves.iter().zip(meshes) {
    let surface = &t.items()[leaf.key.item as usize].surface;
    for tri in mesh.triangles() {
      let corners = tri.map(|i| {
        let (u, v) = mesh.params[i as usize];
        (param_to_f64(u), param_to_f64(v), mesh.positions[i as usize])
      });
      for a in 0..=resolution {
        for b in 0..=resolution - a {
          let l = [a as f64 / r, b as f64 / r, (resolution - a - b) as f64 / r];
          let (mut u, mut v, mut p) = (0., 0., Vec3::new(0., 0., 0.));
          for (c, w) in corners.iter().zip(l) {
            u += c.0 * w;
            v += c.1 * w;
            p += c.2 * w;
          }
          let s = surface.eval(u, v);
          if -s.z <= t.camera().near {
            continue;
          }
          let error = (p - s).length() * t.camera().pixel_scale(s);
          report.samples += 1;
          report.max_pixel_error = report.max_pixel_error.max(error);
          report.samples_over_threshold += (error > threshold) as usize;
        }
      }
    }
  }
  report
}

#[derive(Debug, Default, Clone)]
pub struct WatertightReport {
  /// Iso lines with leaf edges from exactly two sides.
  pub shared_lines: usize,
  /// Iso lines with leaf edges from one side only (face boundary, culled or missing neighbor,
  /// or a neighbor whose boundary control points differ).
  pub open_lines: usize,
  /// Iso lines with leaf edges from more than two sides.
  pub non_manifold_lines: usize,
  /// Vertices present on both sides of a shared line.
  pub shared_vertices: usize,
  /// Shared vertices whose positions differ in any bit (also counted within one side).
  pub mismatched_vertices: usize,
  /// Vertices of one side inside an edge of the other side.
  pub t_junctions: usize,
  /// Max distance of a T junction vertex to the opposite edge, in pixels and relative to the
  /// edge length.
  pub max_t_junction_pixels: f64,
  pub max_t_junction_relative: f64,
  /// Vertices of one side not covered by an edge of the other side.
  pub uncovered_vertices: usize,
  /// The first few mismatched vertices, for debugging.
  pub mismatches: Vec<VertexMismatch>,
}

/// Two leaf vertices at the same point of a line with different positions: `(item, u, v)` of
/// both and their positions.
#[derive(Debug, Clone, Copy)]
pub struct VertexMismatch {
  pub a: (u32, u32, u32),
  pub b: (u32, u32, u32),
  pub positions: [Vec3<f64>; 2],
}

/// A leaf vertex on a line: canonical parameter, position, `(item, u, v)`.
type LineVertex = (u32, Vec3<f64>, (u32, u32, u32));

impl WatertightReport {
  fn mismatch(&mut self, a: &LineVertex, b: &LineVertex) {
    self.mismatched_vertices += 1;
    if self.mismatches.len() < 16 {
      self.mismatches.push(VertexMismatch {
        a: a.2,
        b: b.2,
        positions: [a.1, b.1],
      });
    }
  }
}

#[derive(PartialEq, Eq, Hash)]
enum GlobalLine {
  /// Identified by the instance (one face occurrence) and the canonical boundary curve control
  /// point bits, so patches of a face sharing the boundary meet here. Different faces are out
  /// of scope: their boundary control points can coincide in f64 while the per face origin
  /// makes them differ in the f32 GPU version.
  Boundary(u32, Vec<[u64; 4]>),
  Interior {
    item: u32,
    line: LineKey,
  },
}

#[derive(Default)]
struct LineSide {
  vertices: Vec<LineVertex>,
  /// Consecutive vertex pairs of the leaf edges, ordered by parameter.
  edges: Vec<((u32, Vec3<f64>), (u32, Vec3<f64>))>,
}

/// Collects the boundary vertices of all leaves per iso line and side, and checks that shared
/// vertices are bitwise equal and that every vertex of one side lies on the opposite side's
/// edge polyline.
pub fn check_watertight(t: &mut Tessellator, leaves: &[Leaf]) -> WatertightReport {
  let meshes: Vec<_> = leaves.iter().map(|leaf| t.leaf_mesh(leaf)).collect();
  check_mesh_watertight(t, leaves, &meshes)
}

/// `check_watertight` of leaf meshes from another source (for example read back from the GPU),
/// `meshes[i]` is the grid of `leaves[i]`. `t` only identifies the iso lines.
pub fn check_mesh_watertight(
  t: &mut Tessellator,
  leaves: &[Leaf],
  meshes: &[LeafMesh],
) -> WatertightReport {
  let mut lines: FastHashMap<GlobalLine, FastHashMap<u64, LineSide>> = FastHashMap::default();
  for (leaf, mesh) in leaves.iter().zip(meshes) {
    let k = leaf.k as usize;
    let item = leaf.key.item;
    for (edge, (line, _)) in leaf.key.edges().into_iter().enumerate() {
      let indices: Vec<_> = (0..=k)
        .map(|s| match edge {
          0 => s * (k + 1),
          1 => s * (k + 1) + k,
          2 => s,
          _ => k * (k + 1) + s,
        })
        .collect();
      let curve = t.line_curve(item, line);
      let reversed = curve.reversed;
      let (global, side) = if line.is_patch_boundary() {
        let bits = curve
          .curve
          .control_points
          .iter()
          .map(|p| [p.x, p.y, p.z, p.w].map(f64::to_bits))
          .collect();
        let side = item as u64 * 2 + (line.coord == PARAM_ONE) as u64;
        let instance = t.items()[item as usize].instance;
        (GlobalLine::Boundary(instance, bits), side)
      } else {
        // Leaves on the lower coordinate side have this line as their end edge.
        (GlobalLine::Interior { item, line }, edge as u64 % 2)
      };
      let along = |i: usize| {
        let (u, v) = mesh.params[i];
        let p = match line.axis {
          Axis::U => v,
          Axis::V => u,
        };
        if reversed { PARAM_ONE - p } else { p }
      };
      let mut points: Vec<_> = indices
        .iter()
        .map(|&i| {
          let (u, v) = mesh.params[i];
          (along(i), mesh.positions[i], (item, u, v))
        })
        .collect();
      points.sort_by_key(|p| p.0);
      let entry = lines.entry(global).or_default().entry(side).or_default();
      entry.vertices.extend(points.iter().copied());
      entry.edges.extend(
        points
          .windows(2)
          .map(|w| ((w[0].0, w[0].1), (w[1].0, w[1].1))),
      );
    }
  }

  let mut report = WatertightReport::default();
  let camera = *t.camera();
  for sides in lines.values_mut() {
    for side in sides.values_mut() {
      side.vertices.sort_by_key(|v| v.0);
      side.vertices.dedup_by(|a, b| {
        a.0 == b.0 && {
          if !bitwise_eq(a.1, b.1) {
            report.mismatch(a, b);
          }
          true
        }
      });
      side.edges.sort_by_key(|e| e.0.0);
    }
    match sides.len() {
      1 => report.open_lines += 1,
      2 => {
        report.shared_lines += 1;
        let sides: Vec<_> = sides.values().collect();
        for (a, b) in [(sides[0], sides[1]), (sides[1], sides[0])] {
          for vertex in &a.vertices {
            let &(p, position, _) = vertex;
            if let Ok(i) = b.vertices.binary_search_by_key(&p, |v| v.0) {
              // Counted once, from the first side.
              if std::ptr::eq(a, sides[0]) {
                report.shared_vertices += 1;
                if !bitwise_eq(position, b.vertices[i].1) {
                  report.mismatch(vertex, &b.vertices[i]);
                }
              }
              continue;
            }
            let after = b.edges.partition_point(|e| e.0.0 < p);
            let covering = after.checked_sub(1).map(|i| b.edges[i]);
            match covering {
              Some((e0, e1)) if e0.0 < p && p < e1.0 => {
                let d = e1.1 - e0.1;
                let distance = (position - e0.1).cross(d).length() / d.length();
                report.t_junctions += 1;
                report.max_t_junction_pixels = report
                  .max_t_junction_pixels
                  .max(distance * camera.pixel_scale(position));
                report.max_t_junction_relative =
                  report.max_t_junction_relative.max(distance / d.length());
              }
              _ => report.uncovered_vertices += 1,
            }
          }
        }
      }
      _ => report.non_manifold_lines += 1,
    }
  }
  report
}

fn bitwise_eq(a: Vec3<f64>, b: Vec3<f64>) -> bool {
  a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits() && a.z.to_bits() == b.z.to_bits()
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LeafSetReport {
  pub reference_leaves: usize,
  pub other_leaves: usize,
  /// Leaves with the same key, `k` and `Leaf::trimmed` in both sets.
  pub matched: usize,
  /// Leaves of either set without an equal leaf in the other one.
  pub mismatched: usize,
  /// Over the mismatched leaves, the largest of the smallest decision margin (see
  /// `Tessellator::decision_margin`) along the key path from the root to the leaf. A small
  /// value means every mismatch is explained by a near threshold decision.
  pub max_mismatch_margin: f64,
}

/// Compares the leaf set `other` (for example of the GPU version) with the `reference` leaves
/// of `t`, see `LeafSetReport`.
pub fn compare_leaf_sets(t: &mut Tessellator, reference: &[Leaf], other: &[Leaf]) -> LeafSetReport {
  let as_map = |leaves: &[Leaf]| -> FastHashMap<SubPatchKey, (u32, bool)> {
    leaves
      .iter()
      .map(|leaf| (leaf.key, (leaf.k, leaf.trimmed)))
      .collect()
  };
  let (reference_map, other_map) = (as_map(reference), as_map(other));
  let mut report = LeafSetReport {
    reference_leaves: reference.len(),
    other_leaves: other.len(),
    ..Default::default()
  };
  let mut margins = FastHashMap::default();
  for (leaves, opposite) in [(reference, &other_map), (other, &reference_map)] {
    for leaf in leaves {
      if opposite.get(&leaf.key) == Some(&(leaf.k, leaf.trimmed)) {
        report.matched += 1;
        continue;
      }
      report.mismatched += 1;
      let mut margin = f64::INFINITY;
      for level in 0..=leaf.key.level {
        let shift = leaf.key.level - level;
        let key = SubPatchKey {
          item: leaf.key.item,
          level,
          ix: leaf.key.ix >> shift,
          iy: leaf.key.iy >> shift,
        };
        let m = *margins.entry(key).or_insert_with(|| t.decision_margin(key));
        margin = margin.min(m);
      }
      report.max_mismatch_margin = report.max_mismatch_margin.max(margin);
    }
  }
  // Matched leaves are counted from both sides.
  report.matched /= 2;
  report
}

#[derive(Debug, Default, Clone, Copy)]
pub struct UniformReport {
  pub patches: usize,
  pub triangles: usize,
  /// Patches whose resolution was clamped to `max_resolution`.
  pub clamped: usize,
}

/// Triangle count when every visible patch is drawn as one uniform `k×k` grid (any integer
/// k up to `max_resolution`) meeting the full pixel error and the triangle size limit, with
/// the depth clamped to the near plane. Crack handling is ignored.
pub fn uniform_triangle_count(
  items: &[TessellationItem],
  camera: &Camera,
  config: &TessellationConfig,
  max_resolution: u32,
) -> UniformReport {
  let mut report = UniformReport::default();
  for item in items {
    let bounds = surface_bounds(&item.surface);
    if camera.culls(bounds.min, bounds.max) {
      continue;
    }
    let depth = (-bounds.max.z).max(camera.near);
    let scale = camera.focal_length / depth;
    let k_error = if bounds.grid_error(1) * scale <= config.pixel_error {
      1.
    } else {
      (bounds.grid_coefficient * scale / config.pixel_error)
        .sqrt()
        .ceil()
    };
    let k_size = ((bounds.max - bounds.min).length() * scale / config.max_triangle_size).ceil();
    let k = k_error.max(k_size).max(1.);
    let clamped = k > max_resolution as f64;
    let k = if clamped { max_resolution as f64 } else { k } as usize;
    report.patches += 1;
    report.clamped += clamped as usize;
    report.triangles += 2 * k * k;
  }
  report
}
