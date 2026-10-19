mod gltf;
pub mod mesh;
mod svg;

use std::path::Path;
use std::time::{Duration, Instant};

use fast_hash_collection::FastHashMap;
pub use gltf::*;
use mesh::*;
use rendiation_step_to_bezier::validate::ValidationReport;
use rendiation_step_to_bezier::*;
pub use svg::*;

pub fn convert_file(
  path: impl AsRef<Path>,
  config: &ConvertConfig,
) -> std::io::Result<(ConvertSession, Duration)> {
  let raw = std::fs::read_to_string(path)?;
  let start = Instant::now();
  let session = convert_step(&raw, config);
  Ok((session, start.elapsed()))
}

/// Triangulate every patch once per needed normal orientation and instance it per face
/// occurrence.
pub fn write_session_glb(
  session: &ConvertSession,
  path: &Path,
  config: &TriangulationConfig,
) -> Result<TriangulationStats, Box<dyn std::error::Error>> {
  let mut doc = GltfDoc::new();
  let mut stats = TriangulationStats::default();
  let mut meshes = FastHashMap::default();
  for instance in &session.output.instances {
    for patch_index in instance.patches.clone() {
      let patch = &session.output.patches[patch_index];
      let flip = patch.flip_normal != instance.flip_normal;
      let key = patch_index * 2 + flip as usize;
      if !meshes.contains_key(&key) {
        let mesh = triangulate_patch(patch, flip, config, &mut stats);
        let has_triangles = !mesh.indices.is_empty();
        if has_triangles {
          doc.create_surface_mesh(&mesh, "", key);
        }
        meshes.insert(key, has_triangles);
      }
      if meshes[&key] {
        doc.add_surface_instance(key, instance.transform.into_f32(), "");
      }
    }
  }
  let (root, bin) = doc.into_root();
  write_glb(path, &root, &bin)?;
  Ok(stats)
}

/// Per file summary: face results grouped by surface kind, failures grouped by message, and the
/// validation metrics.
pub fn print_summary(
  name: &str,
  session: &ConvertSession,
  validation: &ValidationReport,
  elapsed: Duration,
) {
  let report = &session.output.report;
  let failed: Vec<_> = report.faces.iter().filter(|f| f.result.is_err()).collect();
  println!(
    "{name}: faces {} ok {} failed {} | patches {} instances {} | tol {:e} | {:.1} ms",
    report.faces.len(),
    report.faces.len() - failed.len(),
    failed.len(),
    session.output.patches.len(),
    session.output.instances.len(),
    report.tol,
    elapsed.as_secs_f64() * 1e3
  );
  if !report.parse_errors.is_empty() {
    println!("  parse errors: {}", report.parse_errors.len());
  }
  if !report.brep_warnings.is_empty() {
    println!("  brep warnings: {}", report.brep_warnings.len());
  }

  let mut kinds: Vec<(&str, usize, usize)> = Vec::new();
  for face in &report.faces {
    let entry = match kinds.iter_mut().find(|k| k.0 == face.surface_kind) {
      Some(entry) => entry,
      None => {
        kinds.push((face.surface_kind, 0, 0));
        kinds.last_mut().unwrap()
      }
    };
    if face.result.is_ok() {
      entry.1 += 1;
    } else {
      entry.2 += 1;
    }
  }
  let kinds: Vec<_> = kinds
    .iter()
    .map(|(k, ok, fail)| format!("{k} {ok}/{}", ok + fail))
    .collect();
  println!("  by surface: {}", kinds.join(", "));

  let fitted: usize = report
    .faces
    .iter()
    .filter_map(|f| f.result.as_ref().ok())
    .map(|s| s.fitted_curve_count)
    .sum();
  println!(
    "  trim curves {} (fitted while building loops: {fitted}) | open loops {} | out of domain {} | non positive area patches {} | max edge deviation {:e} | deviation violations {}",
    validation.trim_curve_count,
    validation.open_loops,
    validation.out_of_domain_curves,
    validation.non_positive_area_patches,
    validation.max_edge_deviation,
    validation.edge_deviation_violations,
  );

  let mut groups: Vec<(String, Vec<u64>)> = Vec::new();
  for face in &failed {
    let Err(e) = &face.result else { continue };
    let key = format!(
      "{} {:?}: {}",
      face.surface_kind,
      e.kind,
      truncate(&e.message, 120)
    );
    match groups.iter_mut().find(|g| g.0 == key) {
      Some(g) => g.1.push(face.step_id),
      None => groups.push((key, vec![face.step_id])),
    }
  }
  groups.sort_by_key(|g| std::cmp::Reverse(g.1.len()));
  for (key, ids) in groups.iter().take(15) {
    let shown: Vec<_> = ids.iter().take(6).map(|id| format!("#{id}")).collect();
    println!("  FAIL x{} {key} [{}]", ids.len(), shown.join(" "));
  }
  for message in validation.messages.iter().take(8) {
    println!("  validate: {message}");
  }
}

fn truncate(s: &str, n: usize) -> String {
  if s.chars().count() <= n {
    s.to_string()
  } else {
    s.chars().take(n).collect::<String>() + "..."
  }
}
