//! Print B-rep resolution statistics for STEP files.
//!
//! ```sh
//! cargo run --release -p step-to-bezier --example brep_stats -- [-v] <file.stp | dir>...
//! ```
//!
//! `-v` also lists every occurrence (body, translation, assembly path).

use std::path::{Path, PathBuf};
use std::time::Instant;

use fast_hash_collection::*;
use rendiation_algebra::*;
use step_reader::step_utils::{is_stp_file, visit_stp_files};
use step_reader::table::{Table, TableParseErrors};
use step_to_bezier::brep::*;

fn main() {
  let mut verbose = false;
  let mut files = Vec::new();
  for arg in std::env::args().skip(1) {
    if arg == "-v" {
      verbose = true;
      continue;
    }
    let path = PathBuf::from(&arg);
    if path.is_dir() {
      visit_stp_files(&path, &mut |p| files.push(p.to_path_buf()));
    } else if is_stp_file(&path) {
      files.push(path);
    } else {
      eprintln!("skipping {arg}: not a STEP file or directory");
    }
  }
  if files.is_empty() {
    eprintln!("usage: brep_stats [-v] <file.stp | dir>...");
    std::process::exit(1);
  }
  for file in files {
    report(&file, verbose);
  }
}

fn report(path: &Path, verbose: bool) {
  println!("== {}", path.display());
  let text = match std::fs::read(path) {
    Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
    Err(e) => {
      println!("  read error: {e}");
      return;
    }
  };
  let parse_start = Instant::now();
  let parsed = Table::from_step(&text, None);
  let parse_time = parse_start.elapsed();
  match &parsed.errors {
    TableParseErrors::EntityErrors(e) if e.is_empty() => {}
    TableParseErrors::EntityErrors(e) => {
      println!("  parse errors: {}", e.len());
      for e in e.iter().take(5) {
        println!("    {e}");
      }
    }
    other => {
      println!("  parse failed: {other:?}");
      return;
    }
  }
  let build_start = Instant::now();
  let model = build_brep_model(&parsed.table);
  let build_time = build_start.elapsed();

  println!(
    "  time: parse {:.1} ms, brep {:.1} ms",
    parse_time.as_secs_f64() * 1e3,
    build_time.as_secs_f64() * 1e3
  );
  println!(
    "  faces {}  edges {}  vertices {}  surfaces {}  curves {}  curves_2d {}  bodies {}  occurrences {}",
    model.faces.len(),
    model.edges.len(),
    model.vertices.len(),
    model.surfaces.len(),
    model.curves.len(),
    model.curves_2d.len(),
    model.bodies.len(),
    model.occurrences.len()
  );
  let faces_per_body: Vec<usize> = model
    .bodies
    .iter()
    .map(|b| b.shells.iter().map(|s| s.len()).sum())
    .collect();
  let instanced_faces: usize = model
    .occurrences
    .iter()
    .map(|o| faces_per_body[o.body.index()])
    .sum();
  let reversed_face_uses = model
    .bodies
    .iter()
    .flat_map(|b| b.shells.iter().flatten())
    .filter(|(_, o)| !o)
    .count();
  println!(
    "  instanced faces {instanced_faces}  shells {}  reversed face uses {reversed_face_uses}",
    model.bodies.iter().map(|b| b.shells.len()).sum::<usize>()
  );
  println!(
    "  uncertainty {:?}  length unit {:?} mm  angle unit {:?} rad",
    model.uncertainty, model.length_unit_mm, model.plane_angle_unit_rad
  );

  let mut surface_kinds = FastHashMap::default();
  for f in &model.faces {
    *surface_kinds
      .entry(surface_kind(&model, f.surface))
      .or_insert(0) += 1;
  }
  println!("  faces per surface: {}", join_counts(&surface_kinds));

  let mut curve_kinds = FastHashMap::default();
  for e in &model.edges {
    *curve_kinds.entry(curve_kind(&model, e.curve)).or_insert(0) += 1;
  }
  println!("  edge curves: {}", join_counts(&curve_kinds));

  let mut pcurve_kinds = FastHashMap::default();
  for p in model.edges.iter().flat_map(|e| &e.pcurves) {
    *pcurve_kinds
      .entry(curve2_kind(&model, p.curve))
      .or_insert(0) += 1;
  }
  let bounds: Vec<_> = model.faces.iter().flat_map(|f| &f.bounds).collect();
  println!(
    "  edges with pcurves {}  pcurves {}  seams {}  same_sense false {}",
    model.edges.iter().filter(|e| !e.pcurves.is_empty()).count(),
    model.edges.iter().map(|e| e.pcurves.len()).sum::<usize>(),
    model.edges.iter().filter(|e| e.is_seam).count(),
    model.edges.iter().filter(|e| !e.same_sense).count(),
  );
  if !pcurve_kinds.is_empty() {
    println!("  pcurve kinds: {}", join_counts(&pcurve_kinds));
  }
  println!(
    "  bounds {}  outer {}  vertex loops {}  orientation false {}  faces same_sense false {}",
    bounds.len(),
    bounds.iter().filter(|b| b.is_outer).count(),
    bounds
      .iter()
      .filter(|b| matches!(b.kind, BoundKind::Vertex(_)))
      .count(),
    bounds.iter().filter(|b| !b.orientation).count(),
    model.faces.iter().filter(|f| !f.same_sense).count(),
  );

  let mut groups: FastHashMap<String, (usize, &str)> = FastHashMap::default();
  for w in &model.warnings {
    groups.entry(normalize(w)).or_insert((0, w)).0 += 1;
  }
  println!("  warnings {}", model.warnings.len());
  let mut groups: Vec<_> = groups.into_values().collect();
  groups.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
  for (count, example) in groups {
    println!("    x{count}: {example}");
  }

  if let Some((min, max)) = world_bounds(&model) {
    println!(
      "  world bounds ({:.3}, {:.3}, {:.3}) .. ({:.3}, {:.3}, {:.3})",
      min.x, min.y, min.z, max.x, max.y, max.z
    );
  }

  if verbose {
    for o in &model.occurrences {
      let m = o.transform;
      println!(
        "    occurrence body #{} at ({:.3}, {:.3}, {:.3}) via {}",
        model.bodies[o.body.index()].step_id,
        m.d1,
        m.d2,
        m.d3,
        o.path
      );
    }
  }
}

/// Bounds of every vertex of every occurrence, in world space.
fn world_bounds(model: &BrepModel) -> Option<(Vec3<f64>, Vec3<f64>)> {
  let mut body_vertices = vec![Vec::new(); model.bodies.len()];
  for (i, body) in model.bodies.iter().enumerate() {
    let mut vs: Vec<_> = body
      .shells
      .iter()
      .flatten()
      .flat_map(|(f, _)| &model.faces[f.index()].bounds)
      .flat_map(|b| match &b.kind {
        BoundKind::Edges(edges) => edges
          .iter()
          .flat_map(|e| {
            let e = &model.edges[e.edge.index()];
            [e.start, e.end]
          })
          .collect(),
        BoundKind::Vertex(v) => vec![*v],
      })
      .collect();
    vs.sort_unstable();
    vs.dedup();
    body_vertices[i] = vs;
  }
  let mut bounds: Option<(Vec3<f64>, Vec3<f64>)> = None;
  for o in &model.occurrences {
    for v in &body_vertices[o.body.index()] {
      let p = o.transform * model.vertices[v.index()].position;
      bounds = Some(match bounds {
        None => (p, p),
        Some((min, max)) => (min.min(p), max.max(p)),
      });
    }
  }
  bounds
}

fn join_counts(m: &FastHashMap<String, usize>) -> String {
  let mut entries: Vec<_> = m.iter().collect();
  entries.sort();
  entries
    .iter()
    .map(|(k, v)| format!("{k} {v}"))
    .collect::<Vec<_>>()
    .join(", ")
}

/// Warning message with numbers replaced, for grouping.
fn normalize(w: &str) -> String {
  let mut out = String::with_capacity(w.len());
  let mut in_number = false;
  for c in w.chars() {
    if c.is_ascii_digit() || (in_number && (c == '.' || c == 'e' || c == '-')) {
      if !in_number {
        out.push('N');
      }
      in_number = true;
    } else {
      in_number = false;
      out.push(c);
    }
  }
  out
}

fn surface_kind(model: &BrepModel, s: SurfaceId) -> String {
  match &model.surfaces[s.index()] {
    SurfaceGeom::Plane(_) => "Plane".into(),
    SurfaceGeom::Cylinder { .. } => "Cylinder".into(),
    SurfaceGeom::Cone { .. } => "Cone".into(),
    SurfaceGeom::Sphere { .. } => "Sphere".into(),
    SurfaceGeom::Torus { .. } => "Torus".into(),
    SurfaceGeom::BSpline(b) if b.weights.is_some() => "RationalBSpline".into(),
    SurfaceGeom::BSpline(_) => "BSpline".into(),
    SurfaceGeom::Extrusion { curve, .. } => format!("Extrusion({})", curve_kind(model, *curve)),
    SurfaceGeom::Revolution { curve, .. } => format!("Revolution({})", curve_kind(model, *curve)),
    SurfaceGeom::Offset { basis, .. } => format!("Offset({})", surface_kind(model, *basis)),
    SurfaceGeom::Unsupported { type_name, .. } => format!("Unsupported({type_name})"),
  }
}

fn curve_kind(model: &BrepModel, c: CurveId) -> String {
  match &model.curves[c.index()] {
    CurveGeom::Line { .. } => "Line".into(),
    CurveGeom::Circle { .. } => "Circle".into(),
    CurveGeom::Ellipse { .. } => "Ellipse".into(),
    CurveGeom::BSpline(b) if b.weights.is_some() => "RationalBSpline".into(),
    CurveGeom::BSpline(_) => "BSpline".into(),
    CurveGeom::Polyline(_) => "Polyline".into(),
    CurveGeom::Trimmed { basis, .. } => format!("Trimmed({})", curve_kind(model, *basis)),
    CurveGeom::Composite(_) => "Composite".into(),
    CurveGeom::Offset { basis, .. } => format!("Offset({})", curve_kind(model, *basis)),
    CurveGeom::Unsupported { type_name, .. } => format!("Unsupported({type_name})"),
  }
}

fn curve2_kind(model: &BrepModel, c: Curve2Id) -> String {
  match &model.curves_2d[c.index()] {
    Curve2Geom::Line { .. } => "Line".into(),
    Curve2Geom::Circle { .. } => "Circle".into(),
    Curve2Geom::Ellipse { .. } => "Ellipse".into(),
    Curve2Geom::BSpline(b) if b.weights.is_some() => "RationalBSpline".into(),
    Curve2Geom::BSpline(_) => "BSpline".into(),
    Curve2Geom::Polyline(_) => "Polyline".into(),
    Curve2Geom::Trimmed { basis, .. } => format!("Trimmed({})", curve2_kind(model, *basis)),
    Curve2Geom::Composite(_) => "Composite".into(),
    Curve2Geom::Unsupported { type_name, .. } => format!("Unsupported({type_name})"),
  }
}
