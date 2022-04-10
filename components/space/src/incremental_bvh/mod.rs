// use std::marker::PhantomData;

// use crate::{bvh::*, utils::TreeBuildOption};

// pub struct IncrementalBVH<B> {
//   inner: PhantomData<B>,
// }

// impl<B: BVHBounding> IncrementalBVH<B> {
//   pub fn new<S: BVHBuildStrategy<B>>(
//     source: impl Iterator<Item = B>,
//     strategy: &mut S,
//     option: &TreeBuildOption,
//   ) -> Self {
//     todo!()
//   }

//   pub fn update(&mut self, item: (InTreeToken, B)) {
//     //
//   }

//   pub fn add(&mut self, item: B) -> InTreeToken {
//     todo!()
//   }

//   pub fn remove(&mut self, token: InTreeToken) {
//     //
//   }
// }

// pub struct InTreeToken {
//   index: usize,
// }

// pub enum SourceMutation<B> {
//   Add(B),
// }
