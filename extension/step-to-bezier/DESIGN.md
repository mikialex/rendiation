# step-to-bezier design

Goal: convert a STEP B-rep (AP203/AP214, parsed by `rendiation-step-reader`) into a list of
**rational Bézier patches**, each carrying **closed loops of rational Bézier trim curves** in the
patch-local `[0,1]²` parameter domain, plus per-occurrence transforms. CPU data conversion only,
no GPU work. Reference target: `temp/step-test/steam-controller.stp` (Creo export, 848 faces, no
pcurves, mostly B-spline geometry).

This document is the contract between modules. Read it fully before touching any module.

## Global conventions

- All computation is `f64`. Output is `f64` too, conversion to `f32` is the consumer's job.
- Homogeneous control points: 2D curves use `Vec3<f64>` = `(x·w, y·w, w)`, 3D curves and surfaces
  use `Vec4<f64>` = `(x·w, y·w, z·w, w)`. The degree is `control_points.len() - 1`.
- Surface control point layout: `cp[v * (degree_u + 1) + u]`.
- Never panic on data derived from the file (workspace uses `panic = "abort"`, one panic kills
  the whole conversion). No `unwrap`/`expect`/`assert!`/`todo!`/`unreachable!` on data paths;
  return `ConvertError`. Panics are allowed only for true internal invariant violations.
- Tolerances are model-space lengths. `ConvertCtx::tol` is the model tolerance (STEP
  `UNCERTAINTY_MEASURE_WITH_UNIT` when present, otherwise a size based fallback). Curve fitting
  targets `ctx.fit_tol` (a fraction of `tol`). Never hard code absolute epsilons in model units;
  parameter-space epsilons must be derived from the relevant scale.
- Do not silently "repair" inconsistent data (padding knot vectors, nudging points). Either
  handle the case correctly per the standard or return an error / push a warning.
- Code style: follow the repo `CLAUDE.md` (no `// ----` separators, no numbered list comments,
  minimal explicit types, `cargo fmt` only on this crate: `cargo fmt -p rendiation-step-to-bezier`
  or `rustfmt` on your files, never a workspace wide `cargo fmt`).

## Pipeline overview

```
STEP text
  -> step-reader Table                       (existing crate, fix gaps there if needed)
  -> brep::BrepModel                         clean resolved B-rep: ids, orientations, f64 geometry
  -> per face (deduplicated by face id):
       surface::build_surface                -> Box<dyn SurfaceModel> (native parameterization)
       curve::build_edge3 (cached per edge)  -> exact 3D Bézier chain of the edge, start->end
       curve::build_edge_pcurve (optional)   -> exact 2D Bézier chain in surface native uv
       face::build_face_trim                 -> SurfaceParamSpace (patch grid) + loops in P space
       trim::trim_cells                      -> per patch cell: loops in cell-local [0,1]^2
  -> output::StepConvertOutput               patches + instances + report
  -> approx::approximate_trims_quadratic     optional pass, all trims as quadratic Béziers
```

## Parameter spaces (the key idea)

Every surface has a **native** parameterization exactly as defined by ISO 10303-42 (plane:
lengths along `x`/`y`; cylinder: `u` angle, `v` axial length; cone: `u` angle, `v` axial length
with radius `R + v·tan(α)`; sphere: `u` longitude, `v` latitude in `[-π/2, π/2]`; torus: `u`
major angle, `v` minor angle; B-spline: knot parameters; extrusion: `u` swept curve parameter,
`v` multiple of the extrusion vector; revolution: `u` rotation angle, `v` generatrix parameter).
Pcurves are given in native coordinates.

Rational Bézier patches can not always reproduce the native parameterization linearly. A 90°
circular arc as a rational quadratic has a parameter `t` that is a nonlinear (but monotone)
function of the angle. So for each face we build a **P space** ("patch space"):

- Per axis there is a `PiecewiseParamMap`: native breakpoints `b0 < b1 < ... < bn`, and for each
  segment a `SegmentMap` (`Affine` or `CircularArc`). The P coordinate of native value `x` in
  segment `i` is `i + local_t(x)`. So P is a grid of unit cells `[i, i+1] × [j, j+1]`.
- Each cell is exactly one rational Bézier patch, and `patch.eval(p - (i, j)) == surface.eval(native(p))`
  holds exactly (up to float rounding).
- `CircularArc` uses the exact inverse of the rational arc parameterization. For an arc of span
  `φ` (`φ <= π/2`), local angle `θ ∈ [0, φ]`: `t = 0.5 + tan((θ − φ/2)/2) / (2·tan(φ/4))`.
- Periodic native axes (angles, closed B-splines when detected) make the map periodic: native
  period `T` corresponds to P period `n` (the number of segments covering one period).
- For B-splines (and swept surfaces whose curve is not a conic) the map is fixed by the knot
  spans (`Affine` per span), with P = 0 at the domain start. For periodic axes the face module
  must shift native coordinates into `[period_start, period_start + period)` before building
  the param space (`SurfaceModel::period_start`); cells with index `>= n` map to span
  `index mod n`. Non periodic bounded axes require the rect inside `natural_domain`. For analytic surfaces the
  map is built **per face** from the face's native bounding range: the angular range is split
  into equal arcs of at most 90°, starting at the face's minimum angle. A face spanning less than
  a full turn therefore never touches a seam.

Consequences:
- A trim curve that is affine-related to native space (any pcurve on a B-spline or a plane,
  pcurves on extrusions/revolutions along the affine axis) maps **exactly** into P after
  splitting at segment breakpoints.
- Iso-lines (`u = const` or `v = const` in native space) are straight lines in P regardless of the
  map, so they are exact too. This covers circles/lines on cylinders, cones, spheres, tori.
- Everything else is **fitted** in P space with a 3D error bound (`ctx.fit_tol`), and marked
  `exact = false` on the output curve.

## Face trimming in P space

`face` produces loops in P space with these guarantees:

- Orientation normalized: material on the **left** in P space (outer loops CCW, holes CW). This
  folds in `OrientedEdge.orientation`, `FaceBound.orientation` and `FaceSurface.same_sense` (if
  `same_sense` is false the loops are reversed and the output patch gets `flip_normal = true`).
  `OrientedFace.orientation` only affects `flip_normal` of the instance, never the loops.
- Consecutive curves share endpoints **exactly** (bitwise), loops are closed **exactly** modulo
  periods: a loop may end at `start + k·period` in a periodic axis (a band loop that wraps around
  the surface, e.g. each circle bounding a full cylinder). `k` is recorded.
- Vertices are mapped once per face so all curves meeting at a vertex agree on its P position.
- Pole handling (sphere poles, cone apex, any degenerate iso-line): when consecutive curves meet
  at a pole with different `u`, a pole segment along the degenerate boundary is inserted.
- Seam edges (one edge used twice in a loop) are just two curves one period apart after
  unwrapping.
- The face domain (which cells exist) covers the unwrapped loops' P bounding box with a margin
  in unbounded native directions, clamped to the natural domain. When a periodic axis has non
  zero net winding, the domain spans the full period, and on the material side it extends to the
  natural boundary or pole. Closed loops that only enclose holes on a closed surface (sphere,
  torus, closed B-spline with degenerate ends) get the full natural domain.
- Fitted curves are split where they cross a cell line, the split points lie exactly on the
  line, and every fitted piece is monotone along its chord with its control polygon inside its
  cell. Vertices within `1e-6` (P) and a negligible model distance of a cell line are snapped
  onto it, so a vertex on a knot line is never seen on both sides of it.
- Exactness: pcurves on affine axes, exact plane projections, pole segments and iso-lines
  proven by the pcurve or by the 3D curve hint (coaxial circles, lines along a cylinder axis /
  extrusion direction / cone generator, sphere and torus meridians) are `exact`. Projected curves
  are fitted with the error measured in model space against the projected edge (`ctx.fit_tol`).

`trim` then works on unit cells only:

- Loop curves (and their copies shifted by whole periods in periodic axes) are split at the cell
  boundary lines using exact Bernstein root finding; crossing points are snapped exactly onto the
  boundary line.
- Each piece is classified as inside / outside / on-boundary. On-boundary pieces are kept only if
  the material side faces into the cell.
- Boundary events (points where kept pieces start or end on the cell boundary) are connected by
  walking the cell boundary **counter-clockwise** from each exit to the next entry (by boundary
  parameter, across all loops). The walked parts become straight boundary curves with source
  `PatchBoundary`.
- Cells crossed by no piece are classified Full or Empty by a first-hit ray test (the side of the
  first curve hit by a ray from a reference point on the cell boundary decides; periodic copies
  included). If every ray misses, the cell is inside when there are no loops, or when there are
  no band loops and the closed loops only enclose holes (negative total area).
- Output loops are in cell-local `[0,1]²` (a pure translation from P).

## Output

See `src/output.rs`. A Full cell produces a single rectangle loop (four `PatchBoundary` lines), an
Empty cell produces no patch. Every trim curve records its source (`Edge { edge, reversed }` or
`PatchBoundary`) so edge topology can be exposed later (crack free tessellation, shared edge
sampling) without changing the pipeline.

## Module ownership

| Module | Content |
|---|---|
| `math` | rational Bézier curves (2D/3D) and surfaces, NURBS decomposition (clamped, unclamped, periodic), conic arcs, piecewise param maps, Bernstein roots, projection, curve fitting |
| `brep` | `Table` -> `BrepModel`: topology with orientations, f64 geometry enums, pcurve references, assembly occurrences, units and uncertainty |
| `curve` | STEP curve geometry -> exact Bézier chains: edge 3D curves trimmed by vertices, 2D pcurves trimmed to the edge, curve param spaces for swept surfaces |
| `surface` | STEP surface geometry -> `SurfaceModel` (native eval, derivatives, projection, periodicity, poles, per-face P space and patches) |
| `face` | per face loop construction in P space (exact mapping, iso-line detection, fitting, unwrapping, orientation, domain) |
| `trim` | cell clipping and loop assembly |
| `pipeline`, `output`, `approx`, `validate` | orchestration, dedup, instances, report, quadratic approximation pass, validation metrics |

## Testing

Each module has unit tests for its own math. End to end, the tester crate
`extension/parametric-step-convert-tester` converts every file under `temp/step-test/` and
`step-generated-sample/`, prints per face status, and runs `validate` (loop closure, curves inside
`[0,1]²`, max 3D distance between edge sourced trims and the original 3D edge, Full/Empty sanity),
and writes glTF (CPU CDT triangulation with all loops as separate constraints) and SVG for
inspection. Target: steam controller converts all 848 faces with no failure and edge deviation
within `tol`.
