# parametric rendering

GPU evaluation of rational Bézier curves and surfaces (compute pipelines with Bernstein basis
evaluation and a de Casteljau fast path for degree <= 3, up to `MAX_GPU_DEGREE`). The CPU side
data is the f64 `RationalBezierSurface` / `RationalBezierCurve3` from `parametric-normalized-model`
(`extension/parametric-data-normalization`), converted to f32 by `bezier_surface_to_gpu` /
`bezier_curve_to_gpu`.
