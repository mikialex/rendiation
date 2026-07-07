use spade::{handles::FixedVertexHandle, ConstrainedDelaunayTriangulation, Point2, Triangulation};

use crate::*;

/// Triangulate a single trimmed Bézier surface.
pub fn triangulate_trimmed_surface(
  trimmed: &TrimmedSurface,
  config: &TriangulationConfig,
) -> MeshData {
  let flat: Vec<QuadraticBezierCurve2d<f32>> =
    trimmed.trim_loops.iter().flatten().cloned().collect();
  let mut mesh = if config.ignore_surface_trim || flat.is_empty() {
    triangulate_untrimmed(&trimmed.surface, config.grid_resolution)
  } else {
    triangulate_trimmed(&trimmed.surface, &flat, config)
  };

  if trimmed.is_back_face {
    for n in &mut mesh.normals {
      *n = *n * -1.0;
    }
    for idx in &mut mesh.indices {
      idx.swap(1, 2);
    }
  }

  mesh
}

/// Per-surface triangulation result.
pub struct MeshData {
  pub positions: Vec<Vec3<f32>>,
  pub normals: Vec<Vec3<f32>>,
  pub uvs: Vec<Vec2<f32>>,
  pub indices: Vec<[u32; 3]>,
}

#[derive(Debug, Clone)]
pub struct TriangulationConfig {
  /// Max chord error when tessellating boundary curves (default 1e-3).
  pub boundary_tolerance: f32,
  /// Interior grid resolution in each parametric direction (default 32).
  pub grid_resolution: usize,
  /// Skip trimming and uniformly sample the full [0,1]² domain (default false).
  pub ignore_surface_trim: bool,
}

impl Default for TriangulationConfig {
  fn default() -> Self {
    Self {
      boundary_tolerance: 0.1,
      grid_resolution: 8,
      ignore_surface_trim: false,
    }
  }
}

pub fn tessellate_curve(curve: &RationalBezierCurve3d<f32>, tolerance: f32) -> Vec<Vec3<f32>> {
  adaptive_tessellate_bezier_curve(curve, tolerance)
}

type SpadePoint = Point2<f64>;
type SpadeCdt = ConstrainedDelaunayTriangulation<SpadePoint>;

/// Regular grid triangulation of the full [0,1]² domain (no trimming).
fn triangulate_untrimmed(surface: &RationalBezierSurface<f32>, resolution: usize) -> MeshData {
  let n = resolution + 1;
  let mut positions = Vec::with_capacity(n * n);
  let mut normals = Vec::with_capacity(n * n);
  let mut uvs = Vec::with_capacity(n * n);

  for j in 0..n {
    let v = j as f32 / resolution as f32;
    for i in 0..n {
      let u = i as f32 / resolution as f32;
      let (p, du, dv) = surface.evaluate_partial(u, v);
      let n = du.cross(dv).normalize();
      positions.push(p);
      normals.push(n);
      uvs.push(Vec2::new(u, v));
    }
  }

  let mut indices = Vec::with_capacity(resolution * resolution * 2);
  for j in 0..resolution {
    for i in 0..resolution {
      let a = (j * n + i) as u32;
      let b = a + 1;
      let c = a + n as u32;
      let d = c + 1;
      indices.push([a, b, d]);
      indices.push([a, d, c]);
    }
  }

  MeshData {
    positions,
    normals,
    uvs,
    indices,
  }
}

/// Triangulate the trimmed region described by 2D boundary curves.
fn triangulate_trimmed(
  surface: &RationalBezierSurface<f32>,
  trim_boundary: &[QuadraticBezierCurve2d<f32>],
  config: &TriangulationConfig,
) -> MeshData {
  // 1. Tessellate all boundary curves into a single closed polygon
  let mut boundary_poly: Vec<Vec2<f32>> = Vec::new();
  for curve in trim_boundary {
    let pts = tessellate_quadratic_bezier_2d(surface, curve, config.boundary_tolerance);
    if boundary_poly.is_empty() {
      boundary_poly.extend(pts);
    } else {
      // Skip duplicate start point (same as previous end)
      let start = if boundary_poly
        .last()
        .is_some_and(|last| (*last - pts[0]).length() < 1e-8)
      {
        1
      } else {
        0
      };
      boundary_poly.extend(&pts[start..]);
    }
  }

  if boundary_poly.len() < 3 {
    return MeshData {
      positions: Vec::new(),
      normals: Vec::new(),
      uvs: Vec::new(),
      indices: Vec::new(),
    };
  }

  // 2. Build CDT with boundary constraints.
  //    Tag each vertex: interior=true, boundary=false.
  let mut cdt = SpadeCdt::new();
  let mut vertex_is_interior: std::collections::HashMap<FixedVertexHandle, bool> =
    std::collections::HashMap::new();

  let mut boundary_handles: Vec<FixedVertexHandle> = Vec::new();
  let mut last_boundary_handle: Option<FixedVertexHandle> = None;
  for &p in &boundary_poly {
    match cdt.insert(SpadePoint::new(p.x as f64, p.y as f64)) {
      Ok(h) => {
        boundary_handles.push(h);
        vertex_is_interior.insert(h, false);
        last_boundary_handle = Some(h);
      }
      Err(_) => {
        // Duplicate point (within spade tolerance) — reuse previous handle
        // so constraint edges still follow the original boundary topology.
        if let Some(h) = last_boundary_handle {
          boundary_handles.push(h);
        }
      }
    }
  }

  // Add constraint edges along the boundary
  if boundary_handles.len() >= 2 {
    let m = boundary_handles.len();
    for i in 0..m {
      let a = boundary_handles[i];
      let b = boundary_handles[(i + 1) % m];
      if cdt.can_add_constraint(a, b) {
        cdt.add_constraint(a, b);
      }
    }
  }

  // 3. Insert interior grid points, tagged as interior
  let res = config.grid_resolution;
  for j in 0..=res {
    let v = j as f32 / res as f32;
    for i in 0..=res {
      let u = i as f32 / res as f32;
      let pt = Vec2::new(u, v);
      if is_inside_boundary(pt, &boundary_poly) {
        if let Ok(h) = cdt.insert(SpadePoint::new(u as f64, v as f64)) {
          vertex_is_interior.insert(h, true);
        }
      }
    }
  }

  // 4. Collect vertex data
  // Build a map from spade vertex handle → mesh index
  let mut mesh_positions: Vec<Vec3<f32>> = Vec::new();
  let mut mesh_normals: Vec<Vec3<f32>> = Vec::new();
  let mut mesh_uvs: Vec<Vec2<f32>> = Vec::new();
  let mut handle_to_idx: std::collections::HashMap<FixedVertexHandle, u32> =
    std::collections::HashMap::new();

  for v in cdt.vertices() {
    let sp: &Point2<f64> = v.as_ref();
    let uv = Vec2::new(sp.x as f32, sp.y as f32);
    let (pos, du, dv) = surface.evaluate_partial(uv.x, uv.y);
    let normal = du.cross(dv).normalize();
    let idx = mesh_positions.len() as u32;
    mesh_positions.push(pos);
    mesh_normals.push(normal);
    mesh_uvs.push(uv);
    handle_to_idx.insert(v.fix(), idx);
  }

  // 5. Extract triangles.
  //    Keep if at least one interior vertex; for pure-boundary triangles,
  //    fall back to centroid winding number.
  let mut indices: Vec<[u32; 3]> = Vec::new();
  for face in cdt.inner_faces() {
    let verts = face.vertices();
    let h0 = verts[0].fix();
    let h1 = verts[1].fix();
    let h2 = verts[2].fix();

    let interior0 = vertex_is_interior.get(&h0).copied().unwrap_or(false);
    let interior1 = vertex_is_interior.get(&h1).copied().unwrap_or(false);
    let interior2 = vertex_is_interior.get(&h2).copied().unwrap_or(false);

    let keep = if interior0 || interior1 || interior2 {
      true
    } else {
      // pure-boundary triangle — use centroid to decide
      let Some(&i0) = handle_to_idx.get(&h0) else {
        continue;
      };
      let Some(&i1) = handle_to_idx.get(&h1) else {
        continue;
      };
      let Some(&i2) = handle_to_idx.get(&h2) else {
        continue;
      };
      let c = (mesh_uvs[i0 as usize] + mesh_uvs[i1 as usize] + mesh_uvs[i2 as usize]) / 3.0;
      is_inside_boundary(c, &boundary_poly)
    };

    if keep {
      let Some(&i0) = handle_to_idx.get(&h0) else {
        continue;
      };
      let Some(&i1) = handle_to_idx.get(&h1) else {
        continue;
      };
      let Some(&i2) = handle_to_idx.get(&h2) else {
        continue;
      };
      indices.push([i0, i1, i2]);
    }
  }

  MeshData {
    positions: mesh_positions,
    normals: mesh_normals,
    uvs: mesh_uvs,
    indices,
  }
}

/// Adaptively tessellate a 2D quadratic Bézier trim curve by recursive
/// subdivision until the 3D chord error is below `tolerance`.
/// Uses `surface.evaluate(u, v)` to map 2D points to 3D for the flatness test.
pub fn tessellate_quadratic_bezier_2d(
  surface: &RationalBezierSurface<f32>,
  curve: &QuadraticBezierCurve2d<f32>,
  tolerance: f32,
) -> Vec<Vec2<f32>> {
  let mut out = vec![curve.start];
  subdivide_qbezier(
    curve.start,
    curve.ctrl,
    curve.end,
    tolerance,
    surface,
    &mut out,
  );
  out.push(curve.end);
  out
}

/// 2D-only tessellation for validation purposes (OOB/gap checks).
/// Does not require a surface; uses 2D flatness test.
pub(crate) fn tessellate_quadratic_bezier_2d_only(
  curve: &QuadraticBezierCurve2d<f32>,
  tolerance: f32,
) -> Vec<Vec2<f32>> {
  let mut out = vec![curve.start];
  subdivide_qbezier_2d(curve.start, curve.ctrl, curve.end, tolerance, &mut out);
  out.push(curve.end);
  out
}

fn subdivide_qbezier_2d(
  p0: Vec2<f32>,
  p1: Vec2<f32>,
  p2: Vec2<f32>,
  tol: f32,
  out: &mut Vec<Vec2<f32>>,
) {
  let mid = (p0 + p2) * 0.5;
  if (p1 - mid).length() < tol {
    return;
  }
  let q1 = (p0 + p1) * 0.5;
  let r1 = (p1 + p2) * 0.5;
  let q2 = (q1 + r1) * 0.5;
  subdivide_qbezier_2d(p0, q1, q2, tol, out);
  out.push(q2);
  subdivide_qbezier_2d(q2, r1, p2, tol, out);
}

fn subdivide_qbezier(
  p0: Vec2<f32>,
  p1: Vec2<f32>,
  p2: Vec2<f32>,
  tol: f32,
  surface: &RationalBezierSurface<f32>,
  out: &mut Vec<Vec2<f32>>,
) {
  let p0_3d = surface.evaluate(p0.x, p0.y);
  let p1_3d = surface.evaluate(p1.x, p1.y);
  let p2_3d = surface.evaluate(p2.x, p2.y);
  let tol_sq = tol * tol;

  if point_to_segment_distance_sq(p1_3d, p0_3d, p2_3d) < tol_sq {
    return;
  }

  let q1 = (p0 + p1) * 0.5;
  let r1 = (p1 + p2) * 0.5;
  let q2 = (q1 + r1) * 0.5;
  subdivide_qbezier(p0, q1, q2, tol, surface, out);
  out.push(q2);
  subdivide_qbezier(q2, r1, p2, tol, surface, out);
}

/// Test if `point` is inside a trim boundary (one closed polygon formed
/// by concatenating all tessellated boundary curves).
fn is_inside_boundary(point: Vec2<f32>, boundary: &[Vec2<f32>]) -> bool {
  winding_number(point, boundary) != 0
}

/// Winding number of `point` relative to a closed polygon.
/// Positive = inside a CCW polygon; 0 = outside.
fn winding_number(point: Vec2<f32>, polygon: &[Vec2<f32>]) -> i32 {
  let mut wn = 0i32;
  let n = polygon.len();
  if n < 3 {
    return 0;
  }
  for i in 0..n {
    let a = polygon[i];
    let b = polygon[(i + 1) % n];
    if a.y <= point.y {
      if b.y > point.y && cross_2d(b - a, point - a) > 0.0 {
        wn += 1;
      }
    } else if b.y <= point.y && cross_2d(b - a, point - a) < 0.0 {
      wn -= 1;
    }
  }
  wn
}

fn cross_2d(a: Vec2<f32>, b: Vec2<f32>) -> f32 {
  a.x * b.y - a.y * b.x
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_test_surface() -> crate::surface::RationalBezierSurface<f32> {
    // A simple bilinear surface (degree 1×1) forming a unit-ish patch
    let cp = vec![
      Vec4::new(0.0, 0.0, 0.0, 1.0),
      Vec4::new(1.0, 0.0, 0.0, 1.0),
      Vec4::new(0.0, 1.0, 0.0, 1.0),
      Vec4::new(1.0, 1.0, 0.0, 1.0),
    ];
    crate::surface::RationalBezierSurface::new(cp, 1, 1)
  }

  #[test]
  fn test_winding_number_square() {
    // CCW square
    let sq = vec![
      Vec2::new(0.0, 0.0),
      Vec2::new(1.0, 0.0),
      Vec2::new(1.0, 1.0),
      Vec2::new(0.0, 1.0),
    ];
    assert_eq!(winding_number(Vec2::new(0.5, 0.5), &sq), 1);
    assert_eq!(winding_number(Vec2::new(-0.1, 0.5), &sq), 0);
    assert_eq!(winding_number(Vec2::new(0.5, -0.1), &sq), 0);
  }

  #[test]
  fn test_tessellate_quadratic_bezier_2d() {
    let curve = QuadraticBezierCurve2d {
      start: Vec2::new(0.0, 0.0),
      ctrl: Vec2::new(0.5, 1.0),
      end: Vec2::new(1.0, 0.0),
    };
    let surface = make_test_surface();
    let pts = tessellate_quadratic_bezier_2d(&surface, &curve, 1e-2);
    assert!(pts.len() >= 2);
    // First and last points should match curve endpoints
    assert!((pts[0] - curve.start).length() < 1e-6);
    assert!((pts[pts.len() - 1] - curve.end).length() < 1e-6);
  }

  #[test]
  fn test_untrimmed_triangulation() {
    let surface = make_test_surface();
    let mesh = triangulate_untrimmed(&surface, 4);
    // (4+1)^2 = 25 vertices, 4*4*2 = 32 triangles
    assert_eq!(mesh.positions.len(), 25);
    assert_eq!(mesh.indices.len(), 32);
    assert_eq!(mesh.uvs.len(), 25);
    assert_eq!(mesh.normals.len(), 25);
  }

  #[test]
  fn test_trimmed_triangulation_simple() {
    let surface = make_test_surface();
    // A triangular trim boundary inside [0,1]²
    let trim = vec![QuadraticBezierCurve2d {
      start: Vec2::new(0.2, 0.2),
      ctrl: Vec2::new(0.5, 0.8),
      end: Vec2::new(0.8, 0.2),
    }];
    let config = TriangulationConfig {
      boundary_tolerance: 1e-2,
      grid_resolution: 8,
      ignore_surface_trim: false,
    };
    let mesh = triangulate_trimmed(&surface, &trim, &config);
    // Should produce some triangles inside the triangular region
    assert!(!mesh.positions.is_empty());
    assert!(!mesh.indices.is_empty());
    // All UVs should be within [0,1]²
    for uv in &mesh.uvs {
      assert!((0.0..=1.0).contains(&uv.x), "u={}", uv.x);
      assert!((0.0..=1.0).contains(&uv.y), "v={}", uv.y);
    }
  }

  #[test]
  fn test_empty_boundary_goes_untrimmed() {
    let surface = make_test_surface();
    let trimmed = TrimmedSurface {
      debug_label: String::new(),
      surface,
      trim_loops: vec![],
      is_back_face: false,
    };
    let config = TriangulationConfig::default();
    let mesh = triangulate_trimmed_surface(&trimmed, &config);
    // Should produce full untrimmed mesh
    let n = config.grid_resolution + 1;
    assert_eq!(mesh.positions.len(), n * n);
  }
}
