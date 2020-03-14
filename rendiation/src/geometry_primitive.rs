use rendiation_math_entity::Line3;
use crate::vertex::Vertex;
use core::marker::PhantomData;
use rendiation_math_entity::Face;

pub trait PrimitiveFromGeometryData {
  fn from_indexed_data(index: &[u16], data: &[Vertex], offset: usize) -> Self;
  fn from_data(data: &[Vertex], offset: usize) -> Self;
}

impl PrimitiveFromGeometryData for Face {
  fn from_indexed_data(index: &[u16], data: &[Vertex], offset: usize) -> Self {
    let a = data[index[offset] as usize].position;
    let b = data[index[offset + 1] as usize].position;
    let c = data[index[offset + 2] as usize].position;
    Face { a, b, c }
  }

  fn from_data(data: &[Vertex], offset: usize) -> Self {
    let a = data[offset].position;
    let b = data[offset + 1].position;
    let c = data[offset + 2].position;
    Face { a, b, c }
  }
}

impl PrimitiveFromGeometryData for Line3 {
  fn from_indexed_data(index: &[u16], data: &[Vertex], offset: usize) -> Self {
    let start = data[index[offset] as usize].position;
    let end = data[index[offset + 1] as usize].position;
    Line3 { start, end }
  }
  fn from_data(data: &[Vertex], offset: usize) -> Self {
    let start = data[offset].position;
    let end = data[offset + 1].position;
    Line3 { start, end }
  }
}

pub trait PrimitiveTopology {
  type Primitive: PrimitiveFromGeometryData;
  const STRIDE: usize;
  const WGPU_ENUM: wgpu::PrimitiveTopology;
}

pub struct TriangleList;

impl PrimitiveTopology for TriangleList {
  type Primitive = Face;
  const STRIDE: usize = 3;
  const WGPU_ENUM: wgpu::PrimitiveTopology = wgpu::PrimitiveTopology::TriangleList;
}

pub struct LineList;

impl PrimitiveTopology for LineList {
  type Primitive = Line3;
  const STRIDE: usize = 2;
  const WGPU_ENUM: wgpu::PrimitiveTopology = wgpu::PrimitiveTopology::LineList;
}

pub struct IndexedPrimitiveIter<'a, T: PrimitiveFromGeometryData> {
  index: &'a [u16],
  data: &'a [Vertex],
  current: i16,
  _phantom: PhantomData<T>,
}

impl<'a, T: PrimitiveFromGeometryData> IndexedPrimitiveIter<'a, T>{
  pub fn new(index: &'a [u16],data: &'a [Vertex])-> Self{
    Self{
      index,
      data,
      current: -1,
      _phantom: PhantomData,
    }
  }
}

impl<'a, T: PrimitiveFromGeometryData> Iterator for IndexedPrimitiveIter<'a, T> {
  type Item = T;

  fn next(&mut self) -> Option<T> {
    self.current+=1;
    if self.current == self.index.len() as i16 {
      None
    } else {
      Some(T::from_indexed_data(self.index, self.data, self.current as usize))
    }
  }
}

pub struct PrimitiveIter<'a, T: PrimitiveFromGeometryData> {
  data: &'a [Vertex],
  current: i16,
  _phantom: PhantomData<T>,
}


impl<'a, T: PrimitiveFromGeometryData> PrimitiveIter<'a, T>{
  pub fn new(data: &'a [Vertex])-> Self{
    Self{
      data,
      current: -1,
      _phantom: PhantomData,
    }
  }
}

impl<'a, T: PrimitiveFromGeometryData> Iterator for PrimitiveIter<'a, T> {
  type Item = T;

  fn next(&mut self) -> Option<T> {
    self.current+=1;
    if self.current == self.data.len() as i16 {
      None
    } else {
      Some(T::from_data( self.data, self.current as usize))
    }
  }
}
