use crate::*;

macro_rules! simple {
  ($Type: ty) => {
    impl IncrementAble for $Type {
      type Delta = Self;

      type Error = ();

      fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
        *self = delta;
        Ok(())
      }

      fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
        cb(self.clone())
      }
    }
  };
}

simple!(bool);
simple!(usize);
simple!(u32);
simple!(u64);
simple!(i32);
simple!(i64);
simple!(f32);
simple!(f64);

simple!(char);
simple!(String);

pub enum VecDelta<T: IncrementAble> {
  Push(T),
  Remove(usize),
  Insert(usize, T),
  Mutate(usize, DeltaOf<T>),
  Pop,
}

impl<T: IncrementAble + Default> IncrementAble for Vec<T> {
  type Delta = VecDelta<T>;
  type Error = (); // todo

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    match delta {
      VecDelta::Push(value) => {
        self.push(value);
      }
      VecDelta::Remove(index) => {
        self.remove(index);
      }
      VecDelta::Insert(index, item) => {
        self.insert(index, item);
      }
      VecDelta::Pop => {
        self.pop().unwrap();
      }
      VecDelta::Mutate(index, delta) => {
        let inner = self.get_mut(index).unwrap();
        inner.apply(delta).unwrap();
      }
    };
    Ok(())
  }

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    for (i, v) in self.iter().enumerate() {
      cb(VecDelta::Push(T::default()));
      v.expand(|d| {
        cb(VecDelta::Mutate(i, d));
      })
    }
  }
}

// struct VectorMap<T: IncrementAble, U: IncrementAble, X> {
//   mapped: X,
//   mapper: Box<dyn Fn(&T) -> U>,
//   map_delta: Box<dyn Fn(&DeltaOf<T>) -> DeltaOf<U>>,
// }

// impl<T, U, X> IncrementAble for VectorMap<T, U, X>
// where
//   T: IncrementAble<Error = ()> ,
//   U: IncrementAble<Error = ()> ,
//   X: IncrementAble<Delta = VecDelta<U>, Error = ()>,
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

// impl<T, X> IncrementAble for VectorFilter<T, X>
// where
//   X: IncrementAble<Delta = VecDelta<T>>,
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
