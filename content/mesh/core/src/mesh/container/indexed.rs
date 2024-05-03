use core::marker::PhantomData;
use std::hash::Hash;

use crate::*;

/// We don't use TryInto<usize, Error: Debug> to express
/// the conversion between the usize and self, because we assume the range of IndexType not
/// exceeds usize. So their conversion is infallible. But the std not impl direct From trait
/// for u32/u16. To keep simplicity, we provide explicit trait function here
///
/// The reason we don't impl from_usize is this should impl by the index container
pub trait IndexType: Copy + Eq + Ord + Hash {
  fn into_usize(self) -> usize;
}
impl IndexType for u32 {
  fn into_usize(self) -> usize {
    self as usize
  }
}
impl IndexType for u16 {
  fn into_usize(self) -> usize {
    self as usize
  }
}

#[derive(Debug, Clone)]
pub enum DynIndexContainer {
  Uint16(Vec<u16>),
  Uint32(Vec<u32>),
}

impl Default for DynIndexContainer {
  fn default() -> Self {
    Self::Uint16(Default::default())
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DynIndex {
  Uint16(u16),
  Uint32(u32),
}

impl IndexType for DynIndex {
  fn into_usize(self) -> usize {
    match self {
      DynIndex::Uint16(i) => i as usize,
      DynIndex::Uint32(i) => i as usize,
    }
  }
}

/// Mark type that indicates index oversized u32 and cannot used in gpu.
#[derive(Debug)]
pub struct IndexOversized;

impl DynIndexContainer {
  pub fn is_u32_buffer(&self) -> bool {
    match self {
      DynIndexContainer::Uint16(_) => false,
      DynIndexContainer::Uint32(_) => true,
    }
  }

  pub fn try_push_index(&mut self, index: usize) -> Result<(), IndexOversized> {
    if index > u32::MAX as usize {
      Err(IndexOversized)
    } else {
      self.push_index(index as u32);
      Ok(())
    }
  }

  pub fn push_index_clamped_u32(&mut self, index: usize) {
    let index = u32::MAX.min(index as u32);
    self.push_index(index)
  }

  fn push_index(&mut self, index: u32) {
    match self {
      DynIndexContainer::Uint16(buffer) => {
        if index > u16::MAX as u32 {
          let buffer = self.check_upgrade_to_u32();
          buffer.push(index)
        } else {
          buffer.push(index as u16)
        }
      }
      DynIndexContainer::Uint32(buffer) => buffer.push(index),
    }
  }

  pub fn check_upgrade_to_u32(&mut self) -> &mut Vec<u32> {
    match self {
      DynIndexContainer::Uint16(buffer) => {
        *self = DynIndexContainer::Uint32(buffer.iter().map(|&i| i as u32).collect());
        self.check_upgrade_to_u32()
      }
      DynIndexContainer::Uint32(buffer) => buffer,
    }
  }
}

impl FromIterator<usize> for DynIndexContainer {
  fn from_iter<T: IntoIterator<Item = usize>>(iter: T) -> Self {
    let mut c = Self::default();
    iter.into_iter().for_each(|i| c.try_push_index(i).unwrap());
    c
  }
}

pub struct DynIndexContainerIter<'a> {
  container: &'a DynIndexContainer,
  current: usize,
  count: usize,
}

impl<'a> Iterator for DynIndexContainerIter<'a> {
  type Item = DynIndex;

  fn next(&mut self) -> Option<Self::Item> {
    if self.current == self.count {
      None
    } else {
      let r = self.container.index_get(self.current).unwrap();
      self.current += 1;
      Some(r)
    }
  }
}

impl<'a> IntoIterator for &'a DynIndexContainer {
  type Item = DynIndex;

  type IntoIter = DynIndexContainerIter<'a>;

  fn into_iter(self) -> Self::IntoIter {
    DynIndexContainerIter {
      container: self,
      current: 0,
      count: match self {
        DynIndexContainer::Uint16(i) => i.len(),
        DynIndexContainer::Uint32(i) => i.len(),
      },
    }
  }
}

impl IndexGet for DynIndexContainer {
  type Output = DynIndex;

  fn index_get(&self, key: usize) -> Option<Self::Output> {
    match self {
      DynIndexContainer::Uint16(i) => DynIndex::Uint16(i.index_get(key).unwrap()),
      DynIndexContainer::Uint32(i) => DynIndex::Uint32(i.index_get(key).unwrap()),
    }
    .into()
  }
}

impl CollectionSize for DynIndexContainer {
  fn len(&self) -> usize {
    match self {
      DynIndexContainer::Uint16(i) => i.len(),
      DynIndexContainer::Uint32(i) => i.len(),
    }
  }
}

/// A indexed mesh that use vertex as primitive;
#[derive(Default, Clone)]
pub struct IndexedMesh<T, U, IU> {
  pub vertex: U,
  pub index: IU,
  _phantom: PhantomData<T>,
}

impl<T, U, IU> From<(U, IU)> for IndexedMesh<T, U, IU> {
  fn from(item: (U, IU)) -> Self {
    IndexedMesh::new(item.0, item.1)
  }
}

impl<T, U, IU> IndexedMesh<T, U, IU> {
  pub fn new(v: U, index: IU) -> Self {
    Self {
      vertex: v,
      index,
      _phantom: PhantomData,
    }
  }

  pub fn as_index_view(&self) -> IndexView<Self> {
    IndexView { mesh: self }
  }
}

impl<T, U, IU> AbstractMesh for IndexedMesh<T, U, IU>
where
  for<'a> IndexView<'a, Self>: AbstractMesh<Primitive = T::Primitive<IU::Output>>,
  U: VertexContainer,
  IU: IndexContainer,
  IU::Output: IndexType,
  U::Output: Copy,
  T: PrimitiveTopologyMeta,
  T::Primitive<IU::Output>: Functor<Unwrapped: IndexType>,
{
  // sadly we can not directly write T::Primitive<U::Output>
  type Primitive = FunctorMapped<T::Primitive<IU::Output>, U::Output>;

  #[inline(always)]
  fn primitive_count(&self) -> usize {
    self.as_index_view().primitive_count()
  }

  #[inline(always)]
  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    let index = self.as_index_view().primitive_at(primitive_index)?;
    index.f_filter_map(|i| self.vertex.index_get(i.into_usize()))
  }

  #[inline(always)]
  unsafe fn primitive_at_unchecked(&self, primitive_index: usize) -> Self::Primitive {
    let index = self.as_index_view().primitive_at_unchecked(primitive_index);
    index.f_map(|i| self.vertex.index_get(i.into_usize()).unwrap_unchecked())
  }
}

pub struct IndexView<'a, T> {
  pub mesh: &'a T,
}

impl<'a, T> std::ops::Deref for IndexView<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.mesh
  }
}

impl<'a, T, U, IU> AbstractMesh for IndexView<'a, IndexedMesh<T, U, IU>>
where
  IU: IndexContainer,
  IU::Output: IndexType,
  T: PrimitiveTopologyMeta,
  T::Primitive<IU::Output>: PrimitiveData<IU>,
{
  type Primitive = T::Primitive<IU::Output>;

  #[inline(always)]
  fn primitive_count(&self) -> usize {
    (self.index.len() + T::STEP - T::STRIDE) / T::STEP
  }

  #[inline(always)]
  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    let index = primitive_index * T::STEP;
    T::Primitive::<IU::Output>::from_data(&self.index, index)
  }

  #[inline(always)]
  unsafe fn primitive_at_unchecked(&self, primitive_index: usize) -> Self::Primitive {
    let index = primitive_index * T::STEP;
    T::Primitive::<IU::Output>::from_data_unchecked(&self.index, index)
  }
}

impl<T, U, IU: CollectionSize> GPUConsumableMeshBuffer for IndexedMesh<T, U, IU> {
  #[inline(always)]
  fn draw_count(&self) -> usize {
    self.index.len()
  }
}
