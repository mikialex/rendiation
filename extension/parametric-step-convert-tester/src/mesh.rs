//! CPU triangulation of trimmed Bézier patches for visual inspection: constrained Delaunay
//! triangulation in the patch domain with every trim loop as its own closed constraint, interior
//! grid points, and the nonzero winding rule over all loops.

use rendiation_algebra::*;
use rendiation_step_to_bezier::TrimmedBezierPatch;
use rendiation_step_to_bezier::math::{RationalBezierCurve2, RationalBezierSurface};
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};

pub struct MeshData {
  pub positions: Vec<Vec3<f32>>,
  pub normals: Vec<Vec3<f32>>,
  pub uvs: Vec<Vec2<f32>>,
  pub indices: Vec<[u32; 3]>,
}

#[derive(Debug, Clone, Copy)]
pub struct TriangulationConfig {
  /// Max 3D chord error when tessellating trim curves.
  pub chord_tolerance: f64,
  /// Interior grid resolution per patch axis.
  pub grid_resolution: usize,
}

impl Default for TriangulationConfig {
  fn default() -> Self {
    Self {
      chord_tolerance: 1e-2,
      grid_resolution: 12,
    }
  }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TriangulationStats {
  /// Constraint edges that intersect or overlap existing constraints (touching or slightly self
  /// intersecting trim loops at the tessellation resolution) and were inserted split.
  pub split_constraints: usize,
}

type Cdt = ConstrainedDelaunayTriangulation<Point2<f64>>;

pub fn triangulate_patch(
  patch: &TrimmedBezierPatch,
  flip_normal: bool,
  config: &TriangulationConfig,
  stats: &mut TriangulationStats,
) -> MeshData {
  let polygons: Vec<Vec<Vec2<f64>>> = patch
    .loops
    .iter()
    .map(|l| {
      let mut polygon = Vec::new();
      for curve in &l.curves {
        tessellate_curve(
          &patch.surface,
          &curve.curve,
          config.chord_tolerance,
          &mut polygon,
        );
      }
      polygon.dedup();
      if polygon.len() > 1 && polygon.first() == polygon.last() {
        polygon.pop();
      }
      polygon
    })
    .filter(|p| p.len() >= 3)
    .collect();

  let mut cdt = Cdt::new();
  for polygon in &polygons {
    let handles: Vec<_> = polygon
      .iter()
      .filter_map(|p| cdt.insert(Point2::new(p.x, p.y)).ok())
      .collect();
    for i in 0..handles.len() {
      let (a, b) = (handles[i], handles[(i + 1) % handles.len()]);
      if a == b {
        continue;
      }
      if cdt.can_add_constraint(a, b) {
        cdt.add_constraint(a, b);
      } else {
        cdt.add_constraint_and_split(a, b, |p| p);
        stats.split_constraints += 1;
      }
    }
  }

  let res = config.grid_resolution.max(1);
  for j in 1..res {
    for i in 1..res {
      let p = Vec2::new(i as f64 / res as f64, j as f64 / res as f64);
      if winding(p, &polygons) != 0 {
        let _ = cdt.insert(Point2::new(p.x, p.y));
      }
    }
  }

  let mut positions = Vec::new();
  let mut normals = Vec::new();
  let mut uvs = Vec::new();
  let mut index_of = vec![u32::MAX; cdt.num_vertices()];
  for v in cdt.vertices() {
    let uv = Vec2::new(v.position().x, v.position().y);
    let (p, du, dv) = patch.surface.eval_with_partials(uv.x, uv.y);
    let mut n = du.cross(dv);
    let len = n.length();
    n = if len > 0. { n / len } else { n };
    if flip_normal {
      n *= -1.;
    }
    index_of[v.fix().index()] = positions.len() as u32;
    positions.push(p.into_f32());
    normals.push(n.into_f32());
    uvs.push(uv.into_f32());
  }

  let mut indices = Vec::new();
  for face in cdt.inner_faces() {
    let [a, b, c] = face.vertices().map(|v| v.fix());
    let pa = cdt.vertex(a).position();
    let pb = cdt.vertex(b).position();
    let pc = cdt.vertex(c).position();
    // Slivers between nearly collinear boundary samples have no domain area but map to large
    // 3D triangles across curved boundaries, never keep them.
    let (e1, e2) = (
      Vec2::new(pb.x - pa.x, pb.y - pa.y),
      Vec2::new(pc.x - pa.x, pc.y - pa.y),
    );
    let longest = e1.length2().max(e2.length2()).max((e2 - e1).length2());
    if (e1.x * e2.y - e1.y * e2.x).abs() <= 1e-9 * longest {
      continue;
    }
    let centroid = Vec2::new((pa.x + pb.x + pc.x) / 3., (pa.y + pb.y + pc.y) / 3.);
    if winding(centroid, &polygons) == 0 {
      continue;
    }
    let tri = [
      index_of[a.index()],
      index_of[b.index()],
      index_of[c.index()],
    ];
    indices.push(if flip_normal {
      [tri[0], tri[2], tri[1]]
    } else {
      tri
    });
  }

  MeshData {
    positions,
    normals,
    uvs,
    indices,
  }
}

/// Append the adaptive tessellation of `curve` (domain points) to `out`, measured by 3D chord
/// error on the surface.
fn tessellate_curve(
  surface: &RationalBezierSurface,
  curve: &RationalBezierCurve2,
  tolerance: f64,
  out: &mut Vec<Vec2<f64>>,
) {
  // Lines are sampled by exact interpolation so iso lines keep their constant coordinate
  // bitwise, rational evaluation rounding would make the samples only nearly collinear.
  let (a, b) = (curve.start(), curve.end());
  let is_line = curve.degree() == 1;
  let at = |t: f64| {
    let uv = if is_line {
      Vec2::new(
        if a.x == b.x {
          a.x
        } else {
          a.x + (b.x - a.x) * t
        },
        if a.y == b.y {
          a.y
        } else {
          a.y + (b.y - a.y) * t
        },
      )
    } else {
      curve.eval(t)
    };
    (uv, surface.eval(uv.x, uv.y))
  };
  let min_depth = if curve.degree() > 1 { 2 } else { 0 };
  let start = at(0.);
  if out.last() != Some(&start.0) {
    out.push(start.0);
  }
  subdivide(&at, (0., start), (1., at(1.)), tolerance, 0, min_depth, out);
}

type Sample = (f64, (Vec2<f64>, Vec3<f64>));

fn subdivide(
  at: &dyn Fn(f64) -> (Vec2<f64>, Vec3<f64>),
  a: Sample,
  b: Sample,
  tolerance: f64,
  depth: usize,
  min_depth: usize,
  out: &mut Vec<Vec2<f64>>,
) {
  const MAX_DEPTH: usize = 10;
  let mid_t = 0.5 * (a.0 + b.0);
  let mid = at(mid_t);
  let chord_mid = (a.1.1 + b.1.1) * 0.5;
  if depth < MAX_DEPTH && (depth < min_depth || (mid.1 - chord_mid).length() > tolerance) {
    subdivide(at, a, (mid_t, mid), tolerance, depth + 1, min_depth, out);
    subdivide(at, (mid_t, mid), b, tolerance, depth + 1, min_depth, out);
  } else {
    out.push(b.1.0);
  }
}

/// Sum of winding numbers of `p` over all closed polygons.
fn winding(p: Vec2<f64>, polygons: &[Vec<Vec2<f64>>]) -> i32 {
  let mut wn = 0;
  for polygon in polygons {
    for i in 0..polygon.len() {
      let a = polygon[i];
      let b = polygon[(i + 1) % polygon.len()];
      let cross = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
      if a.y <= p.y {
        if b.y > p.y && cross > 0. {
          wn += 1;
        }
      } else if b.y <= p.y && cross < 0. {
        wn -= 1;
      }
    }
  }
  wn
}
