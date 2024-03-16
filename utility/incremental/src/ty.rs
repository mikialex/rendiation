use crate::*;

#[macro_export]
macro_rules! clone_self_incremental_base {
  ($Type: ty) => {
    impl $crate::IncrementalBase for $Type {
      type Delta = Self;

      fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
        cb(self.clone())
      }
      fn expand_size(&self) -> Option<usize> {
        Some(1)
      }
    }
    impl $crate::ReversibleIncremental for $Type {
      fn reverse_delta(&self, _delta: &Self::Delta) -> Self::Delta {
        self.clone()
      }
    }
  };
}

#[macro_export]
macro_rules! clone_self_diffable_incremental {
  ($Type: ty) => {
    clone_self_incremental_base!($Type);

    impl $crate::ApplicableIncremental for $Type {
      type Error = ();

      fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
        *self = delta;
        Ok(())
      }

      fn should_apply_hint(&self, delta: &Self::Delta) -> bool {
        self != delta
      }
    }
  };
}

#[macro_export]
macro_rules! clone_self_incremental {
  ($Type: ty) => {
    clone_self_incremental_base!($Type);

    impl $crate::ApplicableIncremental for $Type {
      type Error = ();

      fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
        *self = delta;
        Ok(())
      }
    }
  };
}

clone_self_diffable_incremental!(());

clone_self_diffable_incremental!(bool);
clone_self_diffable_incremental!(usize);
clone_self_diffable_incremental!(u8);
clone_self_diffable_incremental!(i8);
clone_self_diffable_incremental!(u16);
clone_self_diffable_incremental!(i16);
clone_self_diffable_incremental!(u32);
clone_self_diffable_incremental!(u64);
clone_self_diffable_incremental!(i32);
clone_self_diffable_incremental!(i64);
clone_self_diffable_incremental!(f32);
clone_self_diffable_incremental!(f64);

clone_self_diffable_incremental!(char);
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

  fn expand_size(&self) -> Option<usize> {
    self.len().into()
  }
}

pub enum VecMutateError<T: ApplicableIncremental> {
  OutOfBound,
  Mutation(T::Error),
}
impl<T: ApplicableIncremental> Debug for VecMutateError<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::OutOfBound => write!(f, "OutOfBound"),
      Self::Mutation(arg0) => f.debug_tuple("Mutation").field(arg0).finish(),
    }
  }
}

impl<T> ApplicableIncremental for Vec<T>
where
  T: ApplicableIncremental + Default + Clone + Send + Sync + 'static,
{
  type Error = VecMutateError<T>;

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    match delta {
      VecDelta::Push(value) => {
        self.push(value);
      }
      VecDelta::Remove(index) => {
        if self.get(index).is_none() {
          return Err(VecMutateError::OutOfBound);
        }
        self.remove(index);
      }
      VecDelta::Insert(index, item) => {
        if index > self.len() {
          return Err(VecMutateError::OutOfBound);
        }
        self.insert(index, item);
      }
      VecDelta::Pop => return self.pop().map(|_| {}).ok_or(VecMutateError::OutOfBound),
      VecDelta::Mutate(index, delta) => {
        let inner = self.get_mut(index).ok_or(VecMutateError::OutOfBound)?;
        return inner.apply(delta).map_err(VecMutateError::<T>::Mutation);
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

/// Arc is immutable
impl<T: Send + Sync + 'static> SimpleIncremental for std::sync::Arc<T> {
  type Delta = Self;

  fn s_apply(&mut self, delta: Self::Delta) {
    *self = delta;
  }

  fn s_expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(self.clone())
  }
}

pub enum MaybeDeltaRef<'a, T: IncrementalBase> {
  Delta(&'a T::Delta),
  All(&'a T),
}

#[derive(Clone)]
pub enum MaybeDelta<T: IncrementalBase + Send + Sync> {
  Delta(T::Delta),
  All(T),
}

impl<T: IncrementalBase + Send + Sync> MaybeDelta<T> {
  pub fn expand_delta(&self, mut f: impl FnMut(T::Delta)) {
    match self {
      MaybeDelta::Delta(d) => f(d.clone()),
      MaybeDelta::All(v) => v.expand(f),
    }
  }
  pub fn expect_delta(self) -> T::Delta {
    match self {
      MaybeDelta::Delta(v) => v,
      MaybeDelta::All(_) => unreachable!(),
    }
  }
  pub fn expect_all(self) -> T {
    match self {
      MaybeDelta::Delta(_) => unreachable!(),
      MaybeDelta::All(v) => v,
    }
  }
}

pub fn merge_maybe<T>(v: MaybeDelta<T>) -> T
where
  T: IncrementalBase<Delta = T>,
{
  match v {
    MaybeDelta::Delta(d) => d,
    MaybeDelta::All(d) => d,
  }
}
pub fn merge_maybe_ref<T>(v: &MaybeDelta<T>) -> &T
where
  T: IncrementalBase<Delta = T>,
{
  match v {
    MaybeDelta::Delta(d) => d,
    MaybeDelta::All(d) => d,
  }
}
pub fn merge_maybe_mut_ref<T>(v: &mut MaybeDelta<T>) -> &mut T
where
  T: IncrementalBase<Delta = T>,
{
  match v {
    MaybeDelta::Delta(d) => d,
    MaybeDelta::All(d) => d,
  }
}

impl<T: IncrementalBase + Clone + Send + Sync> IncrementalBase for Option<T> {
  type Delta = Option<MaybeDelta<T>>;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    if let Some(inner) = self {
      cb(Some(MaybeDelta::All(inner.clone())));
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
        MaybeDelta::Delta(d) => self.as_mut().unwrap().apply(d)?,
        MaybeDelta::All(v) => *self = Some(v),
      };
    } else {
      *self = None;
    }
    Ok(())
  }
}

impl<T: ReversibleIncremental + Clone + Send + Sync> ReversibleIncremental for Option<T> {
  fn reverse_delta(&self, delta: &Self::Delta) -> Self::Delta {
    match delta {
      Some(delta) => match delta {
        MaybeDelta::Delta(d) => Some(MaybeDelta::Delta(self.as_ref().unwrap().reverse_delta(d))),
        MaybeDelta::All(v) => Some(MaybeDelta::All(v.clone())),
      },
      None => None,
    }
  }
}
