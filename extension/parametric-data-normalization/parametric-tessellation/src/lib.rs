//! CPU reference of the view dependent adaptive tessellation of rational Bézier patches
//! planned in `extension/parametric-rendering/plan.md`, in f64 and structured like the GPU
//! version (level by level work queues, dyadic sub patch keys, no neighbor information).
//!
//! Each (instance, patch) item is split into dyadic sub patches until a sub patch can be drawn
//! as a `k×k` grid (`k` a power of two up to `k_max`) whose triangles deviate from the surface
//! by less than the pixel error, and whose cells are not larger than the triangle size limit.
//! All vertices are evaluated from the original control points at exact dyadic parameters.
//!
//! Edges: neighbors split to different depths would produce T junction cracks. Every iso line
//! of a patch (`u = c` or `v = c`, `c` dyadic) gets a linearization that depends only on the
//! line curve and the camera: the coarsest dyadic segment of the line that can be replaced by
//! at most `k_max` uniform chord pieces within the edge tolerance, searched from the line's
//! root down. A leaf edge must have such a segment above it (otherwise the leaf splits), and
//! its `k` is raised so its edge vertices include every break point of the pieces inside it.
//! Edge vertices inside a piece are interpolated between the positions of the piece ends, so
//! both sides of an edge lay their vertices on the same polyline.
//!
//! The root of the line `u = c` is the cell of the level of `c` (the level where the line first
//! becomes a sub patch edge), not the whole patch side. With that restriction a point lies
//! inside a piece of at most one of its two iso lines, so the vertex position is a pure
//! function of the point: interpolated on that line (the piece ends positioned the same way,
//! recursively), or evaluated exactly. Patch boundary lines are evaluated on a direction
//! canonicalized curve, so neighbor patches with bitwise equal boundary control points agree.
//!
//! Control points are taken relative to a per face origin (`face_origins`) before the
//! transform to camera space, like the f32 GPU version, which keeps the shared boundaries of
//! one face bitwise equal.
//!
//! See `verify` for the checks of the result, and `Tessellator::decision_margin` /
//! `verify::compare_leaf_sets` for comparing a tessellation computed with other rounding.

mod bound;
mod param;
mod tessellate;
pub mod test_scenes;
pub mod trim;
pub mod verify;

pub use bound::*;
pub use param::*;
pub use tessellate::*;

#[cfg(test)]
mod tests;
