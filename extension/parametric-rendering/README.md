# parametric rendering

GPU evaluation of rational Bézier curves and surfaces (compute pipelines with Bernstein basis
evaluation and a de Casteljau fast path for degree <= 3, up to `MAX_GPU_DEGREE`). The CPU side
data is the f64 `RationalBezierSurface` / `RationalBezierCurve3` from `parametric-normalized-model`
(`extension/parametric-data-normalization`), converted to f32 by `bezier_surface_to_gpu` /
`bezier_curve_to_gpu`.

## View dependent adaptive tessellation (`tessellation`)

The GPU version of the CPU reference `parametric-tessellation` (same keys, criteria, `k`
selection, edge linearization and vertex positioning, in f32), see `plan.md` stage 1.

```rust
let mut t = GpuTessellator::new(&gpu, &model, TessellationConfig::default());
t.set_untrimmed_only(true); // optional, like the CPU `camera_space_items` filter
// every frame
let output = t.encode_frame(&gpu, &mut encoder, &camera); // valid after the submit
gpu.submit_encoder(encoder);
let stats = t.last_stats(); // from an earlier frame
```

- `data`: static upload (face origin relative f32 control points once per patch, the item table
  of (instance, patch) occurrences), per frame instance matrices `view · instance.transform ·
  translate(face_origin)` composed in f64 then rounded, and the frame uniform.
- `shader_lib`: shader side ports of the CPU functions (dyadic segments, de Casteljau, sub
  curve and per control point sub patch extraction, the error bounds, iso line curves with the
  canonical boundary direction, `linearization` / `flat_ancestor` / `locate`).
- `passes`: the compute passes of a frame, 77 dispatches with the default config (`max_level`
  10, `k_max` 16). T transform, I init, per level L.a extract / L.b interior / L.c edges /
  L.d decide (L.d also maxes the next level's indirect arguments atomically, replacing a
  separate argument pass), D draw arguments, then the vertex rounds (forward requests, backward
  interpolation) and the final vertex pass with the normals. All sizes come from indirect
  arguments, no readback inside a frame. Every append is capacity checked, overflows drop the
  write and set a flag.
- `tessellator`: `GpuTessellator`, asynchronous statistics (`GpuTessellationStats`) with buffer
  growth after an overflow, `read_back` for tests and tools. Vertex normals come from the
  partials of the face origin relative control points mapped by the instance matrix (camera
  space f32 loses the precision of small patches far away), at poles slightly inside the leaf.
- `output`: `TessellationFrameOutput`, the interface to the draw side (vertex pool, leaf
  records, per `k` bucket leaf lists and `draw_indirect` arguments).

Requirements and limits: up to 10 storage buffers per shader stage (more than the WebGPU
default of 8, fine on native adapters); patch degrees up to `MAX_GPU_DEGREE` (others are
rejected at upload and counted); `k_max <= 16`; at most 65535 workgroups per indirect dispatch
(64 threads per work item dispatch, 256 per vertex dispatch), the capacities are clamped to it.

Tests compare every stage with the CPU reference (`cargo test -p rendiation-parametric-rendering`),
the steam controller comparison is an ignored test:

```sh
cargo test --release -p rendiation-parametric-rendering steam -- --ignored --nocapture
```
