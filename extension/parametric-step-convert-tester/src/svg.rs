use std::path::Path;

use rendiation_parametric_rendering::QuadraticBezierCurve2d;

pub fn curve_color(idx: usize, total: usize) -> (u8, u8, u8) {
  let hue = if total <= 1 { 0.0 } else { (idx as f32 / (total - 1) as f32) * 270.0 };
  let h = hue / 60.0;
  let c = 0.80 * 0.85;
  let x = c * (1.0 - (h % 2.0 - 1.0).abs());
  let m = 0.80 - c;
  let (r, g, b) = match h as u32 % 6 {
    0 => (c, x, 0.0),
    1 => (x, c, 0.0),
    2 => (0.0, c, x),
    3 => (0.0, x, c),
    4 => (x, 0.0, c),
    _ => (c, 0.0, x),
  };
  (((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}

pub fn write_surface_svg(
  dir: &Path,
  label: &str,
  trim_loops: &[Vec<QuadraticBezierCurve2d<f32>>],
) -> Result<(), Box<dyn std::error::Error>> {
  let fname = label.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-");
  let path = dir.join(format!("{fname}.svg"));
  let margin = 20.0;
  let canvas = 540.0;
  let scale = canvas - 2.0 * margin;
  let to_svg = |x: f32, y: f32| -> (f64, f64) {
    (margin + x as f64 * scale as f64, margin + (1.0 - y as f64) * scale as f64)
  };
  let mut svg = String::new();
  svg.push_str(&format!(
    r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {canvas} {canvas}" width="{canvas}" height="{canvas}">"#
  ));
  svg.push('\n');
  svg.push_str(&format!(r#"<rect x="0" y="0" width="{canvas}" height="{canvas}" fill="white"/>"#));
  svg.push('\n');
  let (x0, y0) = to_svg(0.0, 0.0);
  let (x1, y1) = to_svg(1.0, 1.0);
  svg.push_str(&format!(
    r#"<rect x="{x0:.1}" y="{y1:.1}" width="{w:.1}" height="{h:.1}" fill="rgb(242,242,242)"/>"#,
    w = x1 - x0, h = y0 - y1,
  ));
  svg.push('\n');
  let total = trim_loops.iter().map(|l| l.len()).sum::<usize>();
  let mut ci = 0usize;
  for loop_curves in trim_loops {
    for curve in loop_curves {
      let (sx, sy) = to_svg(curve.start.x, curve.start.y);
      let (cx, cy) = to_svg(curve.ctrl.x, curve.ctrl.y);
      let (ex, ey) = to_svg(curve.end.x, curve.end.y);
      let (r, g, b) = curve_color(ci, total);
      ci += 1;
      svg.push_str(&format!(
        r#"<path d="M {sx:.4} {sy:.4} Q {cx:.4} {cy:.4} {ex:.4} {ey:.4}" stroke="rgb({r},{g},{b})" stroke-width="1.2" fill="none"/>"#
      ));
      svg.push('\n');
    }
  }
  svg.push_str("</svg>\n");
  std::fs::write(&path, svg)?;
  Ok(())
}
