//! Loop assembly in native uv: periodic unwrapping, pole segments, shared vertex positions and
//! winding numbers. Input loops are already oriented with the material on the left.

use super::*;

pub(super) struct NativeLoop {
  pub curves: Vec<NativeCurve>,
  /// Whole periods from the loop start to its end per axis (zero for closed loops).
  pub winding: [i32; 2],
}

pub(super) fn reverse_loop(curves: &mut [NativeCurve]) {
  curves.reverse();
  curves.iter_mut().for_each(|c| c.reverse());
}

pub(super) fn assemble_loops(
  fc: &FaceCtx,
  raw: &[(u64, Vec<NativeCurve>)],
) -> ConvertResult<Vec<NativeLoop>> {
  raw
    .iter()
    .map(|(step_id, curves)| assemble_loop(fc, curves.clone()).map_err(|e| e.at(*step_id)))
    .collect()
}

/// `∮ u dv` over the loop samples (shoelace), the signed native area of a closed loop.
pub(super) fn signed_area(l: &NativeLoop) -> f64 {
  let points: Vec<_> = l
    .curves
    .iter()
    .flat_map(|c| c.samples[1..].iter().map(|s| s.uv))
    .collect();
  let n = points.len();
  (0..n)
    .map(|i| {
      let (a, b) = (points[i], points[(i + 1) % n]);
      0.5 * (a.x * b.y - b.x * a.y)
    })
    .sum()
}

fn assemble_loop(fc: &FaceCtx, curves: Vec<NativeCurve>) -> ConvertResult<NativeLoop> {
  let mut out: Vec<NativeCurve> = Vec::with_capacity(curves.len() + 2);
  for mut next in curves {
    if let Some(prev) = out.last_mut() {
      let target = prev.end();
      next.translate(nearest_shift(fc, next.start(), target));
      if let Some(pole_segment) = connect(fc, prev, &mut next, false)? {
        out.push(pole_segment);
      }
    }
    out.push(next);
  }
  let Some(first) = out.first() else {
    return Ok(NativeLoop {
      curves: out,
      winding: [0, 0],
    });
  };

  // Close the loop against a copy of the first curve shifted by whole periods.
  let mut head = first.clone();
  let first_shift = first.shift;
  let last = out.len() - 1;
  let shift0 = nearest_shift(fc, head.start(), out[last].end());
  head.translate(shift0);
  let pole_segment = connect(fc, &mut out[last], &mut head, true)?;
  let shift = head.shift - first_shift;
  let mut winding = [0; 2];
  for (axis, w) in winding.iter_mut().enumerate() {
    if let Some(period) = fc.period[axis] {
      let k = (coord(shift, axis) / period).round();
      if k.abs() > MAX_WINDING {
        return Err(ConvertError::topology(
          None,
          format!("loop winds {k} times around a periodic axis"),
        ));
      }
      *w = k as i32;
    }
  }
  let closed_start = head.start() - shift;
  out[0].set_start(closed_start);
  if let Some(segment) = pole_segment {
    out.push(segment);
  }
  Ok(NativeLoop {
    curves: out,
    winding,
  })
}

const MAX_WINDING: f64 = 64.;

/// Whole period translation moving `from` to the representative nearest to `to`.
fn nearest_shift(fc: &FaceCtx, from: Vec2<f64>, to: Vec2<f64>) -> Vec2<f64> {
  let mut d = Vec2::new(0., 0.);
  for axis in 0..2 {
    if let Some(period) = fc.period[axis] {
      let k = ((coord(to, axis) - coord(from, axis)) / period).round();
      if k.is_finite() {
        set_coord(&mut d, axis, k * period);
      }
    }
  }
  d
}

/// Resolve the shared vertex between `prev` (end) and `next` (start, already shifted to the
/// representative nearest to `prev`). At a pole whose free coordinates differ, `next` is
/// shifted so the pole segment runs with the material on its left, and the segment is
/// returned. Otherwise both ends get the same native position, iso-lines keeping their constant
/// coordinate. `closing`: `next` is the shifted copy of the loop's first curve.
fn connect(
  fc: &FaceCtx,
  prev: &mut NativeCurve,
  next: &mut NativeCurve,
  closing: bool,
) -> ConvertResult<Option<NativeCurve>> {
  let vertex = prev.end3(fc);
  let gap = (vertex - next.start3(fc)).length();
  if exceeds(gap, GAP_FACTOR * fc.ctx.tol) {
    return Err(ConvertError::topology(
      None,
      format!(
        "consecutive loop curves are {gap:e} apart in 3D (tol {:e})",
        fc.ctx.tol
      ),
    ));
  }

  if let Some(pole) = fc.pole_at(vertex, fc.ctx.tol) {
    let (a, b) = (pole.axis.index(), pole.axis.other().index());
    let mut end = prev.end();
    set_coord(&mut end, a, pole.value);
    prev.set_end(end);
    let mut start = next.start();
    set_coord(&mut start, a, pole.value);
    next.set_start(start);

    let eps = free_eps(fc, b);
    let delta = coord(start, b) - coord(end, b);
    let delta = match fc.period[b] {
      Some(period) if delta.abs() > eps => {
        let sign = pole_segment_sign(fc, &pole, prev, next);
        let m = delta.rem_euclid(period);
        let wanted = if m <= eps || m >= period - eps {
          0.
        } else if sign > 0. {
          m
        } else {
          m - period
        };
        next.translate(axis_vec(b, wanted - delta));
        wanted
      }
      _ => delta,
    };
    if delta.abs() <= eps {
      let mut start = next.start();
      set_coord(&mut start, b, coord(end, b));
      next.set_start(start);
      return Ok(None);
    }
    return Ok(Some(pole_segment(
      prev.end(),
      next.start(),
      pole.axis.index(),
    )));
  }

  let mut joint = prev.end();
  let (prev_start, next_start) = (prev.start(), next.start());
  if let Some(axis) = next.iso {
    set_coord(&mut joint, axis, coord(next_start, axis));
  }
  if let Some(axis) = prev.iso {
    set_coord(&mut joint, axis, coord(prev_start, axis));
  }
  if closing && next.iso.is_some() && prev.iso == next.iso {
    // A single iso-line loop (or two collinear ones closing): keep the first curve's value.
    if let Some(axis) = next.iso {
      set_coord(&mut joint, axis, coord(next_start, axis));
    }
  }
  prev.set_end(joint);
  next.set_start(joint);
  Ok(None)
}

/// Native epsilon of the free coordinate at a pole: below it two curves meet at the same
/// point of the pole line.
fn free_eps(fc: &FaceCtx, axis: usize) -> f64 {
  let span = fc.period[axis]
    .or_else(|| fc.surface.natural_domain()[axis].map(|(lo, hi)| hi - lo))
    .unwrap_or(1.);
  1e-9 * span
}

/// Direction (sign of the free coordinate change) of a pole segment with the material on its
/// left: the material lies on the side of the pole line where the neighbor curves come from.
fn pole_segment_sign(fc: &FaceCtx, pole: &Pole, prev: &NativeCurve, next: &NativeCurve) -> f64 {
  let a = pole.axis.index();
  let off_pole = prev
    .samples
    .iter()
    .rev()
    .chain(next.samples.iter())
    .map(|s| coord(s.uv, a) - pole.value)
    .find(|d| d.abs() > free_eps(fc, a));
  let side = match off_pole {
    Some(d) => d.signum(),
    None => {
      let mid = fc.surface.natural_domain()[a].map_or(0., |(lo, hi)| 0.5 * (lo + hi));
      (mid - pole.value).signum()
    }
  };
  if a == 1 { side } else { -side }
}

fn pole_segment(start: Vec2<f64>, end: Vec2<f64>, pole_axis: usize) -> NativeCurve {
  NativeCurve {
    source: TrimCurveSource::Pole,
    geom: NativeGeom::Line,
    chain3: Vec::new(),
    samples: vec![
      Sample {
        s: 0.,
        uv: start,
        pole: true,
      },
      Sample {
        s: 1.,
        uv: end,
        pole: true,
      },
    ],
    shift: Vec2::new(0., 0.),
    iso: Some(pole_axis),
    iso_exact: true,
    iso_dev: [0., 0.],
  }
}
