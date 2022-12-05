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
macro_rules! clone_self_incremental {
  ($Type: ty) => {
    impl $crate::SimpleIncremental for $Type {
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

clone_self_incremental!(());

clone_self_incremental!(bool);
clone_self_incremental!(usize);
clone_self_incremental!(u32);
clone_self_incremental!(u64);
clone_self_incremental!(i32);
clone_self_incremental!(i64);
clone_self_incremental!(f32);
clone_self_incremental!(f64);

clone_self_incremental!(char);
clone_self_incremental!(String);

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
