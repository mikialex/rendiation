//! The actually mesh data container, define how we store the vertex

pub mod indexed;
pub mod none_indexed;

pub use indexed::*;
pub use none_indexed::*;

use crate::group::MeshGroup;

pub trait AbstractMesh {
  type Primitive;

  fn draw_count(&self) -> usize;
  fn primitive_count(&self) -> usize;
  fn primitive_at(&self, primitive_index: usize) -> Self::Primitive;

  fn get_full_group(&self) -> MeshGroup {
    MeshGroup {
      start: 0,
      count: self.draw_count(),
    }
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

  fn primitive_iter_group(&self, group: MeshGroup) -> AbstractMeshIter<'_, Self>
  where
    Self: Sized,
  {
    assert!(group.start <= self.draw_count());
    assert!(group.count <= self.draw_count());

    let step = self.draw_count() / self.primitive_count();

    AbstractMeshIter {
      mesh: self,
      current: group.start,
      count: group.count / step,
    }
  }
}

pub struct AbstractMeshIter<'a, G> {
  mesh: &'a G,
  current: usize,
  count: usize,
}

impl<'a, G: AbstractMesh> Iterator for AbstractMeshIter<'a, G> {
  type Item = G::Primitive;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    if self.current == self.count {
      return None;
    }
    let p = self.mesh.primitive_at(self.current);
    self.current += 1;
    Some(p)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = self.mesh.primitive_count() - self.current;
    (len, Some(len))
  }
}

impl<'a, G: AbstractMesh> ExactSizeIterator for AbstractMeshIter<'a, G> {
  #[inline(always)]
  fn len(&self) -> usize {
    self.mesh.primitive_count() - self.current
  }
}

pub trait AbstractIndexMesh: AbstractMesh {
  type IndexPrimitive;

  fn index_primitive_at(&self, primitive_index: usize) -> Self::IndexPrimitive;

  fn index_primitive_iter(&self) -> AbstractIndexMeshIter<'_, Self>
  where
    Self: Sized,
  {
    AbstractIndexMeshIter {
      mesh: self,
      current: 0,
      count: self.primitive_count(),
    }
  }
  fn index_primitive_iter_group(&self, group: MeshGroup) -> AbstractIndexMeshIter<'_, Self>
  where
    Self: Sized,
  {
    assert!(group.start <= self.primitive_count());
    assert!(group.count <= self.primitive_count());

    let step = self.draw_count() / self.primitive_count();

    AbstractIndexMeshIter {
      mesh: self,
      current: group.start,
      count: group.count / step,
    }
  }
}

pub struct AbstractIndexMeshIter<'a, G> {
  mesh: &'a G,
  current: usize,
  count: usize,
}

impl<'a, G: AbstractIndexMesh> Iterator for AbstractIndexMeshIter<'a, G> {
  type Item = G::IndexPrimitive;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    if self.current == self.count {
      return None;
    }
    let p = self.mesh.index_primitive_at(self.current);
    self.current += 1;
    Some(p)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = self.mesh.primitive_count() - self.current;
    (len, Some(len))
  }
}

impl<'a, G: AbstractIndexMesh> ExactSizeIterator for AbstractIndexMeshIter<'a, G> {
  #[inline(always)]
  fn len(&self) -> usize {
    self.mesh.primitive_count() - self.current
  }
}
