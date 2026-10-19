use std::fmt::Write;
use std::path::Path;

use rendiation_step_to_bezier::brep::FaceId;
use rendiation_step_to_bezier::{StepConvertOutput, TrimCurveSource};

/// One SVG per face: every patch drawn in its P space cell, trim curves colored by source
/// (edges colored per edge id, patch boundaries gray, poles purple), a dot marks each curve
/// start so the loop direction is visible.
pub fn write_face_svgs(output: &StepConvertOutput, dir: &Path) -> std::io::Result<usize> {
  let mut by_face: Vec<(FaceId, Vec<usize>)> = Vec::new();
  for (i, patch) in output.patches.iter().enumerate() {
    match by_face.last_mut() {
      Some((face, list)) if *face == patch.face => list.push(i),
      _ => by_face.push((patch.face, vec![i])),
    }
  }
  for (face, patch_indices) in &by_face {
    let step_id = output
      .report
      .faces
      .iter()
      .find(|f| f.face == *face)
      .map_or(0, |f| f.step_id);
    let svg = face_svg(output, patch_indices);
    std::fs::write(dir.join(format!("face_{step_id}.svg")), svg)?;
  }
  Ok(by_face.len())
}

fn face_svg(output: &StepConvertOutput, patch_indices: &[usize]) -> String {
  let cells: Vec<_> = patch_indices
    .iter()
    .map(|i| output.patches[*i].cell)
    .collect();
  let min_i = cells.iter().map(|c| c.0).min().unwrap_or(0);
  let max_i = cells.iter().map(|c| c.0).max().unwrap_or(0);
  let min_j = cells.iter().map(|c| c.1).min().unwrap_or(0);
  let max_j = cells.iter().map(|c| c.1).max().unwrap_or(0);
  let (w, h) = ((max_i - min_i + 1) as f64, (max_j - min_j + 1) as f64);
  let scale = 900. / w.max(h);
  let margin = 20.;
  let to_svg = |x: f64, y: f64| {
    (
      margin + (x - min_i as f64) * scale,
      margin + (h - (y - min_j as f64)) * scale,
    )
  };

  let mut svg = String::new();
  let (width, height) = (w * scale + 2. * margin, h * scale + 2. * margin);
  let _ = writeln!(
    svg,
    r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width:.0} {height:.0}" width="{width:.0}" height="{height:.0}"><rect width="100%" height="100%" fill="white"/>"#
  );
  for &index in patch_indices {
    let patch = &output.patches[index];
    let (ci, cj) = (patch.cell.0 as f64, patch.cell.1 as f64);
    let (x0, y1) = to_svg(ci, cj + 1.);
    let _ = writeln!(
      svg,
      r#"<rect x="{x0:.2}" y="{y1:.2}" width="{scale:.2}" height="{scale:.2}" fill="rgb(244,244,244)" stroke="rgb(200,200,200)" stroke-width="0.5"/>"#
    );
    for trim_loop in &patch.loops {
      for curve in &trim_loop.curves {
        let (color, dash) = match curve.source {
          TrimCurveSource::Edge { edge, .. } => (edge_color(edge.0), ""),
          TrimCurveSource::PatchBoundary => {
            ("rgb(150,150,150)".to_string(), r#" stroke-dasharray="3,2""#)
          }
          TrimCurveSource::Pole => ("rgb(160,60,200)".to_string(), ""),
        };
        let mut d = String::new();
        const SAMPLES: usize = 32;
        for k in 0..=SAMPLES {
          let p = curve.curve.eval(k as f64 / SAMPLES as f64);
          let (x, y) = to_svg(ci + p.x, cj + p.y);
          let _ = write!(d, "{}{x:.2} {y:.2} ", if k == 0 { "M " } else { "L " });
        }
        let width = if curve.exact { 1.2 } else { 2.0 };
        let _ = writeln!(
          svg,
          r#"<path d="{d}" stroke="{color}" stroke-width="{width}" fill="none"{dash}/>"#
        );
        let start = curve.curve.start();
        let (sx, sy) = to_svg(ci + start.x, cj + start.y);
        let _ = writeln!(
          svg,
          r#"<circle cx="{sx:.2}" cy="{sy:.2}" r="1.6" fill="{color}"/>"#
        );
      }
    }
  }
  svg.push_str("</svg>\n");
  svg
}

fn edge_color(edge: u32) -> String {
  let hue = (edge.wrapping_mul(0x9E37_79B9) % 360) as f64;
  let h = hue / 60.;
  let c = 0.75;
  let x = c * (1. - (h % 2. - 1.).abs());
  let (r, g, b) = match h as u32 {
    0 => (c, x, 0.),
    1 => (x, c, 0.),
    2 => (0., c, x),
    3 => (0., x, c),
    4 => (x, 0., c),
    _ => (c, 0., x),
  };
  format!(
    "rgb({},{},{})",
    (r * 255.) as u8,
    (g * 255.) as u8,
    (b * 255.) as u8
  )
}
