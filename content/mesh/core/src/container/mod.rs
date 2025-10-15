//! The actually mesh data container, define how we store the vertex

mod attributes;
mod common;
mod indexed;
mod none_indexed;

pub use attributes::*;
pub use common::*;
pub use indexed::*;
pub use none_indexed::*;

use crate::*;

// note1: adding for<'a> &'a IU: IntoIterator<Item = IU::Output> in where clause is not useful
// because I don't know why such bound should also be bounded explicitly in impls usages

// note2: adding semantic associate type Vertex instead of super trait's Output is not useful
// because type system don't understand these two types are same. So the impls still requires
// Output's bounds.
pub trait VertexContainer: IndexGet<Output: Copy> + CollectionSize {}
impl<T: IndexGet<Output: Copy> + CollectionSize> VertexContainer for T {}

pub trait IndexContainer: IndexGet<Output: IndexType> + CollectionSize {}
impl<T: IndexGet<Output: IndexType> + CollectionSize> IndexContainer for T {}

/// The abstract mesh is an a random access primitive iterator
pub trait AbstractMesh {
  type Primitive;

  fn primitive_count(&self) -> usize;
  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive>;
  /// ## Safety
  ///
  /// bound checking is skipped
  unsafe fn primitive_at_unchecked(&self, primitive_index: usize) -> Self::Primitive {
    self.primitive_at(primitive_index).unwrap_unchecked()
  }

  fn primitive_iter(&self) -> AbstractMeshIter<'_, Self>
  where
    Self: Sized,
  {
    AbstractMeshIter {
      mesh: self,
      current: 0,
      count: self.primitive_count(),
    }
  }

  /// check the mesh has no out of bound error.
  fn validate_access(&self) -> bool
  where
    Self: Sized,
  {
    self.primitive_iter().count == self.primitive_count()
  }

  /// ## Safety
  ///
  /// bound checking is skipped
  unsafe fn primitive_iter_unchecked(&self) -> AbstractMeshUncheckIter<'_, Self>
  where
    Self: Sized,
  {
    AbstractMeshUncheckIter {
      mesh: self,
      current: 0,
      count: self.primitive_count(),
    }
  }
}

pub struct AbstractMeshIter<'a, G> {
  mesh: &'a G,
  current: usize,
  count: usize,
}

impl<G: AbstractMesh> Iterator for AbstractMeshIter<'_, G> {
  type Item = G::Primitive;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    if self.current == self.count {
      return None;
    }
    let p = self.mesh.primitive_at(self.current);
    self.current += 1;
    p
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = self.mesh.primitive_count() - self.current;
    (len, Some(len))
  }
}

impl<G: AbstractMesh> CollectionSize for AbstractMeshIter<'_, G> {
  #[inline(always)]
  fn len(&self) -> usize {
    self.mesh.primitive_count() - self.current
  }
}

pub struct AbstractMeshUncheckIter<'a, G> {
  mesh: &'a G,
  current: usize,
  count: usize,
}

impl<G: AbstractMesh> Iterator for AbstractMeshUncheckIter<'_, G> {
  type Item = G::Primitive;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    if self.current == self.count {
      return None;
    }
    let p = unsafe { self.mesh.primitive_at_unchecked(self.current) };
    self.current += 1;
    Some(p)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = self.mesh.primitive_count() - self.current;
    (len, Some(len))
  }
}

impl<G: AbstractMesh> CollectionSize for AbstractMeshUncheckIter<'_, G> {
  #[inline(always)]
  fn len(&self) -> usize {
    self.mesh.primitive_count() - self.current
  }
}
