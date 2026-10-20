//! GPU evaluation of rational Bézier curves and surfaces. CPU side data comes from
//! `parametric-normalized-model` (f64), converted to f32 when uploaded.

use parametric_normalized_model::{RationalBezierCurve3, RationalBezierSurface};
use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

pub mod device;
pub mod tessellation;

use device::*;
