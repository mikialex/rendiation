//! The native rect of a face: bounding range of the loops with a margin, full periods where
//! loops wind around, extended to the natural bound or pole on the material side of a net
//! winding, and shifted by whole periods into the base period.

use super::*;

pub(super) fn face_rect(
  fc: &FaceCtx,
  loops: &mut [NativeLoop],
  holes_only: bool,
) -> ConvertResult<NativeRect> {
  let natural = fc.surface.natural_domain();
  let full_surface = loops.is_empty() || holes_only;
  align_loops(fc, loops);
  let bbox = loops_bbox(loops);
  let scale = native_scale(fc, bbox);

  let mut range = [(0., 0.); 2];
  for axis in 0..2 {
    let winds = loops.iter().any(|l| l.winding[axis] != 0);
    range[axis] = match (fc.period[axis], natural[axis]) {
      (Some(period), _) => {
        let start = match bbox {
          Some((lo, _)) if !full_surface => coord(lo, axis),
          _ => fc.surface.period_start()[axis].unwrap_or(0.),
        };
        let partial = bbox
          .filter(|_| !full_surface && !winds)
          .and_then(|(lo, hi)| {
            let (lo, hi) = (coord(lo, axis), coord(hi, axis));
            let m = margin(hi - lo, scale[axis], fc.ctx.tol);
            (hi - lo + 2. * m < period).then_some((lo - m, hi + m))
          });
        partial.unwrap_or((start, start + period))
      }
      (None, Some(domain)) if full_surface => domain,
      (None, domain) => {
        let Some((lo, hi)) = bbox else {
          return Err(ConvertError::topology(
            None,
            "face without edge loops on a surface unbounded along a non periodic axis",
          ));
        };
        let (lo, hi) = (coord(lo, axis), coord(hi, axis));
        let m = margin(hi - lo, scale[axis], fc.ctx.tol);
        clamp_range((lo - m, hi + m), domain)?
      }
    };
  }

  for axis in 0..2 {
    let net: i32 = loops.iter().map(|l| l.winding[axis]).sum();
    if net == 0 {
      continue;
    }
    // Traveling +u the material is on the +v side; traveling +v it is on the -u side.
    let other = 1 - axis;
    let upper = (net > 0) == (axis == 0);
    let bound = material_bound(fc, other, upper, range[other]).ok_or_else(|| {
      ConvertError::topology(
        None,
        format!(
          "loops wind {net} times around native axis {axis} but the surface has no bound on the \
           material side"
        ),
      )
    })?;
    if upper {
      range[other].1 = bound;
    } else {
      range[other].0 = bound;
    }
    if fc.period[other].is_some() {
      return Err(ConvertError::topology(
        None,
        "net winding around a periodic axis of a doubly periodic surface",
      ));
    }
  }

  // Shift periodic axes so the rect starts in the base period.
  let starts = fc.surface.period_start();
  for (axis, r) in range.iter_mut().enumerate() {
    let (Some(period), Some(start)) = (fc.period[axis], starts[axis]) else {
      continue;
    };
    let k = ((r.0 - start) / period).floor();
    if !k.is_finite() {
      return Err(ConvertError::numerical(None, "non finite native rect"));
    }
    if k != 0. {
      let d = -k * period;
      *r = (r.0 + d, r.1 + d);
      let shift = axis_vec(axis, d);
      for l in loops.iter_mut() {
        l.curves.iter_mut().for_each(|c| c.translate(shift));
      }
    }
  }

  for (lo, hi) in range {
    if !(lo.is_finite() && hi.is_finite() && lo < hi) {
      return Err(ConvertError::numerical(
        None,
        format!("invalid face native range {lo}..{hi}"),
      ));
    }
  }
  Ok(NativeRect {
    u: range[0],
    v: range[1],
  })
}

/// Each loop is projected independently, so closed loops may sit in different period
/// representatives: move every closed loop to the representative whose center is nearest to
/// the first loop's center (a hole inside the outer loop is less than half a period away).
fn align_loops(fc: &FaceCtx, loops: &mut [NativeLoop]) {
  let center = |l: &NativeLoop| loops_bbox(std::slice::from_ref(l)).map(|(lo, hi)| (lo + hi) * 0.5);
  let Some(reference) = loops.first().and_then(center) else {
    return;
  };
  for l in loops.iter_mut().skip(1) {
    let Some(c) = center(l) else {
      continue;
    };
    let mut shift = Vec2::new(0., 0.);
    for axis in 0..2 {
      if let Some(period) = fc.period[axis]
        && l.winding[axis] == 0
      {
        let k = ((coord(reference, axis) - coord(c, axis)) / period).round();
        if k.is_finite() {
          set_coord(&mut shift, axis, k * period);
        }
      }
    }
    l.curves.iter_mut().for_each(|c| c.translate(shift));
  }
}

/// Native bounding box of all loop samples.
fn loops_bbox(loops: &[NativeLoop]) -> Option<(Vec2<f64>, Vec2<f64>)> {
  loops
    .iter()
    .flat_map(|l| &l.curves)
    .flat_map(|c| &c.samples)
    .map(|s| s.uv)
    .fold(None, |acc, p| match acc {
      None => Some((p, p)),
      Some((lo, hi)) => Some((
        Vec2::new(lo.x.min(p.x), lo.y.min(p.y)),
        Vec2::new(hi.x.max(p.x), hi.y.max(p.y)),
      )),
    })
}

/// Model space length per native unit along each axis, estimated at the bbox center and
/// corners (the largest value, so margins in model space are not underestimated).
fn native_scale(fc: &FaceCtx, bbox: Option<(Vec2<f64>, Vec2<f64>)>) -> [f64; 2] {
  let Some((lo, hi)) = bbox else {
    return [1., 1.];
  };
  let mut scale = [0f64; 2];
  for p in [
    lo,
    hi,
    Vec2::new(lo.x, hi.y),
    Vec2::new(hi.x, lo.y),
    (lo + hi) * 0.5,
  ] {
    let (_, su, sv) = fc.surface.eval_with_partials(p);
    scale[0] = scale[0].max(su.length());
    scale[1] = scale[1].max(sv.length());
  }
  scale.map(|s| if s.is_finite() && s > 0. { s } else { 1. })
}

/// Rect margin of an axis: a fraction of the span plus a few model tolerances.
fn margin(span: f64, scale: f64, tol: f64) -> f64 {
  let m = MARGIN_FRACTION * span + MARGIN_TOLS * tol / scale;
  if m.is_finite() && m > 0. {
    m
  } else {
    1e-9 * (1. + span.abs())
  }
}

const MARGIN_FRACTION: f64 = 0.02;
const MARGIN_TOLS: f64 = 4.;

fn clamp_range(range: (f64, f64), domain: Option<(f64, f64)>) -> ConvertResult<(f64, f64)> {
  let Some((d0, d1)) = domain else {
    return Ok(range);
  };
  let (lo, hi) = (range.0.max(d0), range.1.min(d1));
  if lo < hi {
    Ok((lo, hi))
  } else {
    Err(ConvertError::topology(
      None,
      format!(
        "face loops {}..{} outside the surface domain {d0}..{d1}",
        range.0, range.1
      ),
    ))
  }
}

/// The natural bound (or pole) of `axis` on the given side of `range`.
fn material_bound(fc: &FaceCtx, axis: usize, upper: bool, range: (f64, f64)) -> Option<f64> {
  if let Some((lo, hi)) = fc.surface.natural_domain()[axis] {
    return Some(if upper { hi } else { lo });
  }
  let poles = fc.poles.iter().filter(|p| p.axis.index() == axis);
  if upper {
    poles
      .map(|p| p.value)
      .filter(|v| *v > range.0)
      .reduce(f64::min)
  } else {
    poles
      .map(|p| p.value)
      .filter(|v| *v < range.1)
      .reduce(f64::max)
  }
}
