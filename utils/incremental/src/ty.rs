use crate::*;

pub struct SimpleMutator<'a, T: Incremental> {
  pub inner: &'a mut T,
  pub collector: &'a mut dyn FnMut(T::Delta),
}

impl<'a, T: Incremental> MutatorApply<T> for SimpleMutator<'a, T> {
  fn apply(&mut self, delta: T::Delta) {
    (self.collector)(delta.clone());
    self.inner.apply(delta).unwrap()
  }
}

#[macro_export]
macro_rules! simple {
  ($Type: ty) => {
    impl SimpleIncremental for $Type {
      type Delta = Self;

      fn s_apply(&mut self, delta: Self::Delta) {
        *self = delta;
      }

      fn s_expand(&self, mut cb: impl FnMut(Self::Delta)) {
        cb(self.clone())
      }
    }
  };
}

simple!(());

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

#[derive(Clone)]
pub enum VecDelta<T: Incremental> {
  Push(T),
  Remove(usize),
  Insert(usize, T),
  Mutate(usize, DeltaOf<T>),
  Pop,
}

impl<T: Incremental + Default + Clone + 'static> Incremental for Vec<T> {
  type Delta = VecDelta<T>;
  type Error = (); // todo

  type Mutator<'a> = SimpleMutator<'a, Self>;

  fn create_mutator<'a>(
    &'a mut self,
    collector: &'a mut dyn FnMut(Self::Delta),
  ) -> Self::Mutator<'a> {
    SimpleMutator {
      inner: self,
      collector,
    }
  }

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
    for v in self.iter().cloned() {
      cb(VecDelta::Push(v));
    }
  }
}

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

pub trait SimpleIncremental {
  type Delta: Clone;

  fn s_apply(&mut self, delta: Self::Delta);
  fn s_expand(&self, cb: impl FnMut(Self::Delta));
}

impl<T: SimpleIncremental> Incremental for T {
  type Delta = <T as SimpleIncremental>::Delta;

  type Error = ();

  type Mutator<'a> = SimpleMutator<'a, Self>
  where
    Self: 'a;

  fn create_mutator<'a>(
    &'a mut self,
    collector: &'a mut dyn FnMut(Self::Delta),
  ) -> Self::Mutator<'a> {
    SimpleMutator {
      inner: self,
      collector,
    }
  }

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    self.s_apply(delta);
    Ok(())
  }

  fn expand(&self, cb: impl FnMut(Self::Delta)) {
    self.s_expand(cb)
  }
}

/// not mutable
impl<T> Incremental for std::rc::Rc<T> {
  type Delta = Self;

  type Error = ();

  type Mutator<'a> = SimpleMutator<'a, Self>
  where
    Self: 'a;

  fn create_mutator<'a>(
    &'a mut self,
    collector: &'a mut dyn FnMut(Self::Delta),
  ) -> Self::Mutator<'a> {
    SimpleMutator {
      inner: self,
      collector,
    }
  }

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    *self = delta;
    Ok(())
  }

  fn expand(&self, _: impl FnMut(Self::Delta)) {}
}

/// should used for sum type
#[derive(Clone)]
pub enum DeltaOrEntire<T: Incremental> {
  Delta(T::Delta),
  Entire(T),
}

impl<T: Incremental + Clone> Incremental for Option<T> {
  type Delta = Option<DeltaOrEntire<T>>;

  type Error = T::Error;

  type Mutator<'a> = SimpleMutator<'a, Self>
  where
    Self: 'a;

  fn create_mutator<'a>(
    &'a mut self,
    collector: &'a mut dyn FnMut(Self::Delta),
  ) -> Self::Mutator<'a> {
    SimpleMutator {
      inner: self,
      collector,
    }
  }

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    if let Some(d) = delta {
      match d {
        DeltaOrEntire::Delta(d) => self.as_mut().unwrap().apply(d)?,
        DeltaOrEntire::Entire(v) => *self = Some(v),
      };
    } else {
      *self = None;
    }
    Ok(())
  }

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    if let Some(inner) = self {
      cb(Some(DeltaOrEntire::Entire(inner.clone())));
    } else {
      cb(None)
    }
  }
}

trait InteriorMutable<T> {
  fn mutate(&self, f: impl FnMut(&mut T));
}
