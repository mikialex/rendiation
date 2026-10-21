# parametric-tessellation

CPU reference (f64) of the view dependent adaptive tessellation of rational Bézier patches
planned in `extension/parametric-rendering/plan.md` (stage 0). It is structured like the GPU
version: level by level work queues of dyadic sub patch keys, no neighbor information, every
vertex evaluated from the original control points at exact dyadic parameters.

- `Tessellator::run`: split each (instance, patch) until it can be drawn as a `k×k` grid within
  the pixel error and the triangle size limit, and its four edges have a linearization.
- `Tessellator::leaf_mesh`: the leaf grid, boundary vertices on the shared edge linearization.
- `bound`: rigorous error bounds for rational patches and curves (bilinear / chord bound via
  `(H - L·w) / w_min`, second derivative bound via the quotient rule).
- `verify`: dense sampled screen space error, edge consistency (bitwise shared vertices, T
  junction offsets), triangle count of a per patch uniform grid for comparison. The error and
  edge checks also take leaf meshes from another source (`measure_mesh_error`,
  `check_mesh_watertight`), and `compare_leaf_sets` compares another leaf set with the
  reference, explaining mismatches by `Tessellator::decision_margin` (the smallest relative
  margin of the threshold decisions of a key). This is how the f32 GPU version is checked.
  Shared boundaries are only identified within one instance (face occurrence), cross face
  edges are out of scope (their control points may coincide in f64 but not after the per face
  origin in f32).
- `Tessellator::leaf_meshes_by_rounds`: the vertex positions computed in the structure of the
  GPU vertex passes (forward request rounds, backward interpolation), same bits as `leaf_mesh`.
- `camera_space_items` takes the control points relative to their face origin
  (`face_origins`, `relative_to_origin`) and maps them by `item_to_camera`, the data the GPU
  uploads in f32. One origin per face keeps shared boundaries bitwise equal.
- `test_scenes`: synthetic patches and models, also used by the GPU tests.
- `trim`: trim preprocessing and the CPU reference of the per fragment trimming
  (`extension/parametric-rendering/plan-trim.md`): conversion to quadratic Béziers, the f32
  winding kernel (Slug's root eligibility code), exchangeable acceleration structures
  (`TrimAccel`: `AllSegments`, `BandGrid`), the GPU encoding (`encode_trims`) and a polyline
  ground truth. The ignored test `steam_statistics` validates and measures a real model.

Differences from the first plan draft, see the crate docs in `src/lib.rs` for the details:

- An edge is linearized by up to `k_max` uniform chord pieces instead of one chord, otherwise
  every leaf with a curved edge must split until that edge is straight and `k > 1` never helps.
  A leaf raises its `k` to contain the break points inside its edges.
- The root segment of the iso line `u = c` is the cell of the level of `c`. This makes "the
  point lies inside a piece of one of its two iso lines" unique, so the vertex position is a pure
  function of the point (interpolated between the recursively positioned piece ends, or exact).
  Piece ends evaluated exactly instead would crack where a T junction point is a piece end of
  the perpendicular line.
- Rational patches use the same bounds as polynomial ones (no correction factor needed).
- No back face culling yet (normal cones not implemented).

Try it on a model through the CLI:

```sh
cargo run --release -p parametric-normalization-cli -- convert model.pnm -o view.glb --camera fit:4 --verify
```
