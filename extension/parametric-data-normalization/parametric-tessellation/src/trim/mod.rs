//! Trim preprocessing and the CPU reference of the per fragment trimming planned in
//! `extension/parametric-rendering/plan-trim.md`.
//!
//! Trim curves are converted to polynomial quadratic Béziers in the patch domain
//! (`trim_quadratics`). A sample is classified by the winding number of the `+u` ray against
//! them (`crossing`, Slug's root eligibility code). Which segments a sample has to look at is
//! decided by an exchangeable acceleration structure (`TrimAccel`): `AllSegments` (reference)
//! or `BandGrid` (uniform bands plus a pre classification grid).
//!
//! GPU encoding (`encode_trims`), two buffers:
//!
//! - Segment pool, `EncodedSegment` = two vec4 `(p0.x, p0.y, p1.x, p1.y)`,
//!   `(p2.x, p2.y, max_u, bitcast<f32>(flags))`. The segments of a patch are consecutive, loop by
//!   loop, each starting bitwise where the previous one ends.
//! - u32 table: `TRIM_HEADER_WORDS` words per patch (indexed by patch index, all patches of the
//!   model) `segment_offset, segment_count, accel_offset, kind`, followed by the structure data.
//!   `TRIM_KIND_INSIDE` (untrimmed, or a trim covering the domain) and `TRIM_KIND_OUTSIDE` have
//!   no segments and no data. The `BandGrid` data layout is documented on the type.
//!
//! Fragment: clamp `uv` (`clamp_uv`), read the header, run the point query of its kind with the
//! kernel, inside iff the winding is not zero (`TrimEncoding::winding`). Leaf classification:
//! `TrimEncoding::classify_rect`.

mod accel;
mod encode;
mod kernel;
mod quadratic;
mod reference;

pub use accel::*;
pub use encode::*;
pub use kernel::*;
pub use quadratic::*;
pub use reference::*;

#[cfg(test)]
mod tests;
