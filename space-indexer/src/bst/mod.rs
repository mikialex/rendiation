use rendiation_math::{Vec2, Vec3};
use std::{marker::PhantomData, ops::Range};

struct Binary;
struct Quad;
struct Oc;

pub trait BinarySpaceTree<const N: usize> {
  type Center;
}

impl BinarySpaceTree<2> for Binary {
  type Center = f32;
}

impl BinarySpaceTree<4> for Quad {
  type Center = Vec2<f32>;
}

impl BinarySpaceTree<8> for Oc {
  type Center = Vec3<f32>;
}

pub struct BSTNode<T: BinarySpaceTree<N>, const N: usize> {
  phantom: PhantomData<T>,
  pub center: T::Center,
  pub primitive_range: Range<usize>,
  pub depth: usize,
  pub self_index: usize,
  pub child: Option<[usize; N]>,
}
