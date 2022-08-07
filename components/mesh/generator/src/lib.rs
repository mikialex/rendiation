use std::marker::PhantomData;
use std::ops::Range;

use rendiation_algebra::*;
use rendiation_renderable_mesh::mesh::{IndexedMesh, LineList, TriangleList};

mod builder;
pub use builder::*;
mod builtin;
pub use builtin::*;
mod parametric;
pub use parametric::*;
mod combination;
pub use combination::*;
mod primitive;
pub use primitive::*;
