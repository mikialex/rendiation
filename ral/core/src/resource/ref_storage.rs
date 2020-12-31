// struct RefStorage<T, U>{
//     data: T,
//     referenced_by: Vec<Handle<U>>,
//   }

// struct RefStorageArena<T, U>{
//   data: Vec<RefStorage<T, U>>,
//   free_list: Vec<usize>,
//   on_item_mutated: Box<dyn FnMut(&T)>
// }

// struct RefStorageHandle<T, U>{
//   index: usize,
//   phantom1: PhantomData<T>,
//   phantom2: PhantomData<U>,
// }

// impl<T, U> RefStorageArena<T, U> {
//   pub fn insert(){
//     todo!()
//   }

//   pub fn update(handle: RefStorageHandle<T, U>) ->  &mut T {
//     todo!()
//   }
// }
