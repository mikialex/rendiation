mod operator;
pub use operator::*;
mod ctx;
pub use ctx::*;
mod ty;
pub use ty::*;
mod sbt;
pub use sbt::*;
mod pipeline;
pub use pipeline::*;

use crate::*;

// #[test]
// fn t() {
//   ray_ctx_from_declared_payload_input()
//     .then_trace_ray(|state, ctx| {
//       //
//     })
//     .then(|state, ctx| {
//       //
//     })
//     .then_trace_ray(|state, ctx| {
//       //
//     })
//     .then(|state, ctx| {
//       //
//     })
// }
