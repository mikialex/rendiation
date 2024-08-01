use crate::*;

pub trait RawBufferLoadStore {
  fn u32_size_count() -> u32;
  fn load_from_u32_buffer(target: StorageNode<[u32]>, offset: Node<u32>) -> Self;
  fn store_into_u32_buffer(self, target: StorageNode<[u32]>, offset: Node<u32>);
}

macro_rules! raw_buffer_primitive_impl {
  ($Type: ty) => {
    impl RawBufferLoadStore for Node<$Type> {
      fn u32_size_count() -> u32 {
        1
      }

      fn load_from_u32_buffer(target: StorageNode<[u32]>, offset: Node<u32>) -> Self {
        target.index(offset).load().bitcast()
      }

      fn store_into_u32_buffer(self, target: StorageNode<[u32]>, offset: Node<u32>) {
        target.index(offset).store(self.bitcast())
      }
    }
  };
}

raw_buffer_primitive_impl!(u32);
raw_buffer_primitive_impl!(i32);
raw_buffer_primitive_impl!(f32);

// impl<u32> RawBufferLoadStore for Node<Vec2<u32>> {
//   fn u32_size_count() -> u32 {
//     2
//   }

//   fn load_from_u32_buffer(target: StorageNode<[u32]>, offset: Node<u32>) -> Self {
//     target.index(offset).load().bitcast()
//   }

//   fn store_into_u32_buffer(self, target: StorageNode<[u32]>, offset: Node<u32>) {
//     target.index(offset).store(self.bitcast())
//   }
// }
