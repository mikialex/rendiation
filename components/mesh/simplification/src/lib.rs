#![feature(array_chunks)]
#![allow(clippy::disallowed_types)] // we have already used custom hasher

use std::collections::{hash_map::Entry, HashMap};

use rendiation_algebra::*;
use rendiation_geometry::{Box3, Positioned};

mod qem;
use qem::*;

mod hasher;
use hasher::*;

mod remap;
use remap::*;

mod edge_collapse;
use edge_collapse::*;

mod connectivity;
use connectivity::*;

const INVALID_INDEX: u32 = u32::MAX;

pub use edge_collapse::{simplify_by_edge_collapse, EdgeCollapseConfig, EdgeCollapseResult};
pub use remap::{generate_vertex_remap, remap_vertex_buffer};
