# step-to-bezier TODO

Handoff notes for continuing the STEP → trimmed rational Bézier conversion. Read `DESIGN.md`
first (module contract), then the module docs (`src/*/mod.rs`).

## Current state

- Scope: CPU data conversion only (no GPU work). Output: `ParametricNormalizedModel` (crate
  `parametric-normalized-model`, serializable as `.pnm`) with rational Bézier patches, closed rational Bézier trim loops in patch-local `[0,1]²` (material on the left), and
  per occurrence instances. `approx::approximate_trims_quadratic` is the optional pass to
  polynomial quadratics.
- All modules are implemented: `math`, `brep`, `curve`, `surface`, `face`, `trim`, `pipeline`,
  `validate`, `approx`. `cargo test -p step-to-bezier`: 122 passed, 2 ignored (the Bézier type tests moved to
  `parametric-normalized-model`).
- Results (release): `temp/step-test/steam-controller.stp` converts all 848 faces in ~5.6 s:
  17387 patches, 71138 trim curves (1862 fitted, the rest exact), 0 open loops, 0 out of
  domain curves, 0 non positive area patches, max edge deviation 2.8e-3 (tol 9.9e-3). All 21
  real files in `temp/step-test/step_tests` pass. Solid closure (Σ ½∮ r×dr over all trim
  images ≈ 0) verified on all real files, so face orientation and region choice are right.
- Old implementation (`extension/parametric-rendering/src/step`, `mesh`, `validation`) was
  deleted. `STEP_CONVERSION_REVIEW.md` in the repo root reviews that old code (historical).

## How to run

```sh
# all test files, summary per file
cargo run --release -p parametric-normalization-cli -- batch
# one file, with glTF (CPU CDT triangulation), per face SVG trim plots and the pnm round trip check
cargo run --release -p parametric-normalization-cli -- batch --gltf --svg --pnm temp/step-test/steam-controller.stp
# output: extension/parametric-data-normalization/cli/test-output/{*.glb, *.pnm, svg/<file>/face_<step id>.svg}
# single conversion: STEP or pnm in, pnm and/or glb out
cargo run --release -p parametric-normalization-cli -- convert in.stp -o out.pnm -o out.glb
# brep level statistics (entity kinds, units, warnings)
cargo run --release -p step-to-bezier --example brep_stats -- temp/step-test/
```

Workspace uses `panic = "abort"`: never panic on file data. Run `cargo clippy` per package
(`-p step-to-bezier`, `-p parametric-normalized-model`, `-p parametric-normalization-cli`) and `rustfmt` only on
touched files.

## Open work

### Correctness / coverage

- [ ] **Pole crossing in the middle of an edge.** `face/native.rs` only handles poles at curve
  endpoints. A projected curve passing through a pole (sphere pole, cone apex, degenerate
  B-spline boundary) mid edge is not split and fails during fitting. Split the curve at the pole
  and insert a pole segment like at endpoints. No test file hits it yet.
- [ ] **Trim loop self intersection.** Fitted curves near sharp corners can cross each other
  slightly while each stays within `fit_tol` (steam face #47734 and its mirror #90748: 12.6°
  corner next to a 0.02 mm edge). Add a self intersection metric to `validate.rs`, then make
  fitting aware of the neighbouring curves (e.g. tighter tolerance near short edges / sharp
  corners, or fit with tangent constraints at shared vertices).
- [ ] **Validate the quadratic pass.** `approx::approximate_trims_quadratic` was never run on
  real data. Add a `--quadratic` flag to the cli `batch` command, check closure, domain, deviation, and
  self intersection after the pass.
- [ ] **Unsupported geometry** (returns `ConvertError::unsupported`, face skipped):
  `OFFSET_SURFACE`, `OFFSET_CURVE_3D`, `HYPERBOLA`, `PARABOLA`, `DEGENERATE_TOROIDAL_SURFACE`,
  `RECTANGULAR_TRIMMED_SURFACE`, `CURVE_BOUNDED_SURFACE`, pcurve circles/ellipses under non
  uniform angle scaling. Offset surfaces need an approximation strategy (not exactly rational).
- [ ] **Surface pole detection gaps** (`surface/`): revolution surfaces whose conic generatrix
  touches the axis do not report the pole; horn torus (`R = 0`) does not report poles.
- [ ] **Parameter conventions to double check against ISO 10303-42** (`curve/resolve.rs`):
  `TRIMMED_CURVE` uses the ISO parameterization `s = t − t1` (OCCT keeps the basis parameter);
  `COMPOSITE_CURVE` concatenates segment parameter lengths. Only matters for pcurves on swept
  surfaces whose generatrix is a trimmed/composite curve. Changing it is local to
  `trimmed_chain` / the composite builder.
- [ ] **brep limitations** (`brep/`): `CARTESIAN_TRANSFORMATION_OPERATOR_3D` in SRRWT /
  `MAPPED_ITEM` targets is skipped with a warning; one global unit set (inconsistent contexts
  only warn); `POLY_LOOP` (faceted breps) skipped; `DEFINITIONAL_REPRESENTATION` with more than
  one item skipped; faces referenced only from GD&T representations are not collected.
- [ ] **trim limitations** (`trim/`): non manifold regions (two holes or a hole and the outer
  loop sharing an edge segment) are not supported; periodic copy enumeration has no upper bound
  guard for absurd coordinates.
- [ ] **Edge pcurve rejection threshold.** `build_edge_pcurve` rejects pcurves whose image is
  more than 10·tol from the 3D edge (the face module then falls back to projection with a
  warning). Revisit if real data shows sloppier pcurves.
- [ ] **Tolerance policy** (`pipeline.rs::resolve_tolerance`): STEP uncertainty, clamped below by
  `1e-7 · bbox diagonal` (io1-tc declares 1e-13 while vertices are off by ~3e-9). Revisit with
  more files.

### Test data

- [ ] The generated samples `step-generated-sample/{sphere,cone,extrusion}.stp` are non
  conforming (sphere: DEGREE units but pcurves in radians and colatitude; cone: vertices placed
  with `R − v·tanα`, semi angle in radians under DEGREE units; extrusion: edge #50 contradicts
  `same_sense`). Regenerate them correctly or delete them.
- [ ] More real files: pcurves on periodic surfaces in degree units, large assemblies, files
  with offset surfaces, NX/CATIA exports with `SEAM_CURVE`.
- [ ] Decide whether to keep the `#[ignore]` real file tests (`curve/real_files.rs`, surface
  steam test in `surface/tests.rs`) or move them into the cli crate.

### Performance

- [ ] `math/fit.rs` dominates (~55% of face time on steam): per sample `bernstein_basis` Vec
  allocation and per iteration matrix allocation in the SDM loop. Fixed size arrays (degree is
  small) should roughly halve total time. Projection is ~15%.

### CLI (`extension/parametric-data-normalization/cli`)

- [ ] glTF for steam is ~126 MB: make the interior grid resolution adaptive to the patch 3D
  size instead of a fixed 12×12 per patch.
- [ ] 71 constraint edges are inserted split by spade on steam (touching or slightly crossing
  trim loops at the tessellation resolution, see self intersection item above).

### Future features

- [ ] **Edge topology output.** Every trim curve carries `TrimCurveSource::Edge { edge,
  reversed }`; expose shared edge information (both faces' curves per B-rep edge) so a consumer
  can tessellate shared edges consistently (crack free).
- [ ] **GPU rendering of trimmed patches.** `rendiation-parametric-rendering` only evaluates
  untrimmed Bézier curves / surfaces on GPU (`bezier_surface_to_gpu` / `bezier_curve_to_gpu`
  convert the f64 model types); trim loops are not uploaded or used yet.
- [ ] **pnm size.** Steam is ~22 MB (compact MessagePack, uncompressed f64). Consider
  compression or deduplicating shared surfaces if it matters.

### Housekeeping

- [ ] Pre-existing clippy warnings in `extension/step-reader` (derive generated
  `redundant_field_names`, std `HashMap` in tables).
