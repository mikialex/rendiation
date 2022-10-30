pub trait IncrementAble {
  type Delta;
  type DeltaResult;

  fn apply(&mut self, delta: Self::Delta) -> Self::DeltaResult;
}

pub type DeltaOf<T> = <T as IncrementAble>::Delta;

pub struct IncrementInstance<T: IncrementAble> {
  value: T,
  deltas: Vec<T::Delta>,
}

impl<T: IncrementAble> IncrementInstance<T> {
  pub fn push(&mut self, delta: T::Delta) {
    self.deltas.push(delta)
  }

  pub fn flush(&mut self) {
    self.deltas.drain(..).for_each(|d| {
      self.value.apply(d);
    })
  }
}

pub enum VecDelta<T> {
  Push(T),
  Remove(usize),
  Pop,
}

impl<T> IncrementAble for Vec<T> {
  type Delta = VecDelta<T>;
  type DeltaResult = ();

  fn apply(&mut self, delta: Self::Delta) {
    match delta {
      VecDelta::Push(value) => self.push(value),
      VecDelta::Remove(index) => {
        self.remove(index);
      }
      VecDelta::Pop => {
        self.pop();
      }
    }
  }
}

struct VectorMap<T, U, X> {
  mapped: X,
  mapper: Box<dyn Fn(&T) -> U>,
}

impl<T, U, X> IncrementAble for VectorMap<T, U, X>
where
  X: IncrementAble<Delta = VecDelta<U>, DeltaResult = ()>,
{
  type Delta = VecDelta<T>;
  type DeltaResult = ();
  fn apply(&mut self, delta: VecDelta<T>) {
    match delta {
      VecDelta::Push(value) => self.mapped.apply(VecDelta::Push((self.mapper)(&value))),
      VecDelta::Remove(index) => {
        self.mapped.apply(VecDelta::Remove(index));
      }
      VecDelta::Pop => {
        self.mapped.apply(VecDelta::Pop);
      }
    }
  }
}

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

impl IncrementAble for f32 {
  type Delta = Self;
  type DeltaResult = ();

  fn apply(&mut self, delta: Self::Delta) {
    *self = delta
  }
}

impl IncrementAble for bool {
  type Delta = Self;
  type DeltaResult = ();

  fn apply(&mut self, delta: Self::Delta) {
    *self = delta
  }
}

// struct Test {
//   a: f32,
//   b: bool,
// }

// enum TestIncremental {
//   A(DeltaOf<f32>),
//   B(DeltaOf<bool>),
// }

// impl IncrementAble for Test {
//   type Delta = TestIncremental;

//   fn apply(&mut self, delta: Self::Delta) {
//     match delta {
//       TestIncremental::A(v) => self.a.apply(v),
//       TestIncremental::B(v) => self.b.apply(v),
//     }
//   }
// }
