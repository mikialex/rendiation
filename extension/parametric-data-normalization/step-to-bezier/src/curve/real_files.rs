//! Edge / pcurve statistics over real STEP files (`temp/step-test`). Run with
//! `cargo test -p step-to-bezier --lib curve::real_files -- --ignored --nocapture`.

use std::path::Path;

use fast_hash_collection::FastHashMap;
use step_reader::step_utils::visit_stp_files;
use step_reader::table::Table;

use super::*;
use crate::surface::build_surface;

#[test]
#[ignore]
fn real_file_edges() {
  let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../temp/step-test");
  let mut files = Vec::new();
  visit_stp_files(&root, &mut |p| files.push(p.to_path_buf()));
  files.sort();
  for file in files {
    let Ok(bytes) = std::fs::read(&file) else {
      continue;
    };
    let text = String::from_utf8_lossy(&bytes);
    let table = Table::from_step(&text, None).table;
    let model = build_brep_model(&table);
    let tol = model.uncertainty.filter(|t| *t > 0.).unwrap_or(1e-6);
    let ctx = ConvertCtx::new(tol, 0.1, 3);
    let mut errors = FastHashMap::default();
    let (mut ok, mut degenerate, mut p_ok, mut p_iso) = (0, 0, 0, 0);
    let mut max_dev = 0f64;
    for (i, edge) in model.edges.iter().enumerate() {
      let id = EdgeId(i as u32);
      let e3 = match build_edge3(&model, id, &ctx) {
        Ok(e) => e,
        Err(e) => {
          let key = format!("edge3: {}", e.message.split(" (").next().unwrap_or(""));
          errors.entry(key).or_insert((0, e.to_string())).0 += 1;
          continue;
        }
      };
      ok += 1;
      if e3.segments.is_empty() {
        degenerate += 1;
        continue;
      }
      for p in &edge.pcurves {
        let Ok(surface) = build_surface(&model, p.surface, &ctx) else {
          continue;
        };
        match build_edge_pcurve(&model, id, p.curve, surface.as_ref(), &e3, &ctx) {
          Ok(e2) => {
            p_ok += 1;
            p_iso += e2.iso.is_some() as usize;
            for seg in &e2.segments {
              for k in 0..=8 {
                let q = surface.eval(seg.eval(k as f64 / 8.));
                let d = closest_point_on_chain(&e3.segments, q).map_or(0., |r| r.2);
                max_dev = max_dev.max(d / tol);
              }
            }
          }
          Err(e) => {
            let key = format!("pcurve: {}", e.message.split(" (").next().unwrap_or(""));
            errors.entry(key).or_insert((0, e.to_string())).0 += 1;
          }
        }
      }
    }
    println!(
      "{}: edges {} ok {ok} degenerate {degenerate}, pcurves ok {p_ok} iso {p_iso}, max pcurve dev {max_dev:.3e} tol",
      file.file_name().unwrap_or_default().to_string_lossy(),
      model.edges.len()
    );
    let mut errors: Vec<_> = errors.into_iter().collect();
    errors.sort();
    for (key, (count, sample)) in errors {
      let short: String = key.chars().take(90).collect();
      println!("    {count:5} {short}    e.g. {sample}");
    }
  }
}
