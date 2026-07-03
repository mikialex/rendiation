#![feature(type_alias_impl_trait)]

use std::ops::Range;

use rendiation_algebra::*;

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
use rendiation_mesh_core::*;

pub type AttributesMeshBuilder =
  IndexedMeshBuilder<GroupedMesh<IndexedMesh<TriangleList, Vec<CommonVertex>, DynIndexContainer>>>;
pub type AttributesLineMeshBuilder =
  IndexedMeshBuilder<GroupedMesh<IndexedMesh<LineList, Vec<CommonVertex>, DynIndexContainer>>>;

/// helper fn to quick build attribute mesh
pub fn build_attributes_mesh(f: impl FnOnce(&mut AttributesMeshBuilder)) -> AttributesMeshData {
  let mut builder = AttributesMeshBuilder::default();
  f(&mut builder);
  let mesh = builder.finish();
  mesh.mesh.primitive_iter().collect()
}

/// helper fn to quick build non-indexed attribute mesh.
///
/// Unlike [build_attributes_mesh], vertices are **not** deduplicated
pub fn build_attributes_mesh_non_indexed(
  f: impl FnOnce(&mut AttributesMeshBuilder),
) -> AttributesMeshData {
  let mut builder = AttributesMeshBuilder::default();
  f(&mut builder);
  let group_mesh = builder.finish();
  let indexed = group_mesh.mesh;

  AttributesMeshData::collect_non_indexed(indexed.primitive_iter())
}
