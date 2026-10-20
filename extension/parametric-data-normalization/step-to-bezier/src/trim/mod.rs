//! Clip P space loops against the unit cells of the patch grid. Pure 2D, no knowledge of
//! surfaces. See `DESIGN.md`, section "Face trimming in P space".
//!
//! Per cell: the relevant periodic copies of the loops are assembled into chains (band loops
//! become open chains of consecutive copies whose ends lie outside the cell), split at the four
//! boundary lines with the crossing points snapped exactly onto the lines, and classified piece
//! by piece. Kept runs are either closed cycles or fragments from an entry to an exit on the
//! cell boundary; fragments are joined by walking the boundary counter-clockwise from each exit
//! to the next entry over all fragments. Cells without boundary events decide whether their
//! boundary is in the material with a first-hit ray test.

mod cell;
mod chain;
mod clip;
mod connect;
mod geom;
mod ray;
#[cfg(test)]
mod tests;

use cell::*;
use chain::*;
use clip::*;
use connect::*;
use ray::*;
use rendiation_algebra::*;

use crate::face::PLoop;
use crate::*;

pub enum CellTrim {
  /// The cell lies outside the face, no patch is emitted.
  Empty,
  /// Closed loops in cell-local `[0,1]²`, material on the left. A cell completely inside the
  /// face yields one rectangle loop of four `PatchBoundary` lines.
  Trimmed(Vec<TrimLoop>),
}

#[derive(Debug, Clone, Copy)]
pub struct TrimParams {
  /// P space epsilon (cells have unit size): points closer than this to a cell boundary line
  /// are on it.
  pub p_eps: f64,
}

impl Default for TrimParams {
  fn default() -> Self {
    Self { p_eps: 1e-9 }
  }
}

/// Trim every requested cell. Result is parallel to `cells`.
pub fn trim_cells(
  loops: &[PLoop],
  p_period: [Option<f64>; 2],
  cells: &[(usize, usize)],
  params: &TrimParams,
) -> ConvertResult<Vec<CellTrim>> {
  if !(params.p_eps.is_finite() && params.p_eps > 0. && params.p_eps < 0.01) {
    return Err(ConvertError::invalid(
      None,
      format!("trim: p_eps {} out of range", params.p_eps),
    ));
  }
  let periods = validate_periods(p_period)?;
  let loops = prepare_loops(loops, periods, params.p_eps)?;
  cells
    .iter()
    .map(|index| trim_cell(&loops, periods, Cell::new(*index, params.p_eps)))
    .collect()
}

/// Copies whose bounding box stays farther than this from the cell can not contribute.
const NEIGHBORHOOD_MARGIN: f64 = 1e-3;

fn trim_cell(
  loops: &[PreparedLoop],
  periods: [Option<f64>; 2],
  cell: Cell,
) -> ConvertResult<CellTrim> {
  let region = geom::bbox_expand(cell.bbox(), NEIGHBORHOOD_MARGIN + 16. * cell.eps);
  let mut runs = CellRuns::default();
  for chain in region_chains(loops, periods, region) {
    clip_chain(&cell, &chain, &mut runs);
  }
  let mut out = if runs.fragments.is_empty() {
    if boundary_in_material(&cell, loops, periods)? {
      vec![rectangle_loop(&cell)]
    } else {
      Vec::new()
    }
  } else {
    connect_fragments(&cell, runs.fragments)?
  };
  out.extend(runs.cycles);
  if out.is_empty() {
    return Ok(CellTrim::Empty);
  }
  let offset = Vec2::new(-cell.min.x, -cell.min.y);
  let loops = out
    .into_iter()
    .map(|curves| TrimLoop {
      curves: curves
        .into_iter()
        .map(|c| TrimCurve {
          curve: c.curve.translate(offset),
          source: c.source,
          exact: c.exact,
        })
        .collect(),
    })
    .collect();
  Ok(CellTrim::Trimmed(loops))
}
