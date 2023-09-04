#![feature(array_chunks)]
#![allow(clippy::too_many_arguments)]

use rendiation_algebra::*;
use rendiation_geometry::*;

mod bounding;
mod spatial_adjacency_clustering;

use bounding::*;
pub use spatial_adjacency_clustering::*;
