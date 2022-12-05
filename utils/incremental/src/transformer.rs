// struct VectorMap<T: Incremental, U: Incremental, X> {
//   mapped: X,
//   mapper: Box<dyn Fn(&T) -> U>,
//   map_delta: Box<dyn Fn(&DeltaOf<T>) -> DeltaOf<U>>,
// }

// impl<T, U, X> Incremental for VectorMap<T, U, X>
// where
//   T: Incremental<Error = ()> ,
//   U: Incremental<Error = ()> ,
//   X: Incremental<Delta = VecDelta<U>, Error = ()>,
// {
//   type Delta = VecDelta<T>;
//   type Error = ();
//   fn apply(&mut self, delta: VecDelta<T>) -> Result<(), Self::Error> {
//     match delta {
//       VecDelta::Push(value) => self.mapped.apply(VecDelta::Push((self.mapper)(&value))),
//       VecDelta::Remove(index) => self.mapped.apply(VecDelta::Remove(index)),
//       VecDelta::Pop => self.mapped.apply(VecDelta::Pop),
//       VecDelta::Insert(index, value) => self
//         .mapped
//         .apply(VecDelta::Insert(index, (self.mapper)(&value))),
//       VecDelta::Mutate(index, delta) => self
//         .mapped
//         .apply(VecDelta::Mutate(index, (self.map_delta)(&delta))),
//     }
//   }
// }

// struct VectorFilter<T, X> {
//   mapped: X,
//   raw_max: usize,
//   filtered_index: std::collections::HashSet<usize>,
//   filter: Box<dyn Fn(&T) -> bool>,
// }

// impl<T, X> Incremental for VectorFilter<T, X>
// where
//   X: Incremental<Delta = VecDelta<T>>,
// {
//   type Delta = VecDelta<T>;
//   fn apply(&mut self, delta: VecDelta<T>) {
//     match delta {
//       VecDelta::Push(value) => {
//         if (self.filter)(&value) {
//           self.mapped.apply(VecDelta::Push(value));
//         } else {
//           self.filtered_index.insert(self.raw_max);
//         }
//         self.raw_max += 1;
//       }
//       VecDelta::Remove(index) => {
//         if self.filtered_index.remove(&index) {
//           self.mapped.apply(VecDelta::Remove(todo!()));
//         }
//         self.raw_max -= 1
//       }
//       VecDelta::Pop => {
//         if self.filtered_index.remove(&self.raw_max) {
//           self.mapped.apply(VecDelta::Pop);
//         }
//         self.raw_max -= 1
//       }
//     }
//   }
// }
