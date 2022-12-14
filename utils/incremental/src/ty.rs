use crate::*;

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
pub enum VecDelta<T: IncrementalBase> {
  Push(T),
  Remove(usize),
  Insert(usize, T),
  Mutate(usize, DeltaOf<T>),
  Pop,
}

impl<T> IncrementalBase for Vec<T>
where
  T: IncrementalBase + Default + Clone + Send + Sync + 'static,
{
  type Delta = VecDelta<T>;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    for v in self.iter().cloned() {
      cb(VecDelta::Push(v));
    }
  }
}

impl<T> ApplicableIncremental for Vec<T>
where
  T: ApplicableIncremental + Default + Clone + Send + Sync + 'static,
{
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
}

pub trait SimpleIncremental {
  type Delta: Clone + Send + Sync;

  fn s_apply(&mut self, delta: Self::Delta);
  fn s_expand(&self, cb: impl FnMut(Self::Delta));
}

impl<T: SimpleIncremental + Send + Sync + 'static> IncrementalBase for T {
  type Delta = <T as SimpleIncremental>::Delta;

  fn expand(&self, cb: impl FnMut(Self::Delta)) {
    self.s_expand(cb)
  }
}

impl<T: SimpleIncremental + Send + Sync + 'static> ApplicableIncremental for T {
  type Error = ();

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    self.s_apply(delta);
    Ok(())
  }
}

/// not mutable
impl<T: Send + Sync + 'static> SimpleIncremental for std::sync::Arc<T> {
  type Delta = Self;

  fn s_apply(&mut self, delta: Self::Delta) {
    *self = delta;
  }

  fn s_expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(self.clone())
  }
}

/// should used for sum type
#[derive(Clone)]
pub enum DeltaOrEntire<T: IncrementalBase + Send + Sync> {
  Delta(T::Delta),
  Entire(T),
}

impl<T: IncrementalBase + Clone + Send + Sync> IncrementalBase for Option<T> {
  type Delta = Option<DeltaOrEntire<T>>;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    if let Some(inner) = self {
      cb(Some(DeltaOrEntire::Entire(inner.clone())));
    } else {
      cb(None)
    }
  }
}

impl<T: ApplicableIncremental + Clone + Send + Sync> ApplicableIncremental for Option<T> {
  type Error = T::Error;

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
}

trait InteriorMutable<T> {
  fn mutate(&self, f: impl FnMut(&mut T));
}
