use crate::*;

impl<T: IncrementAble + Clone> IncrementAble for Arena<T> {
  type Delta = ArenaDelta<T>;

  type Error = ();

  type Mutator<'a> = ArenaMutator<'a, T>
  where
    Self: 'a;

  fn create_mutator<'a>(
    &'a mut self,
    collector: &'a mut dyn FnMut(Self::Delta),
  ) -> Self::Mutator<'a> {
    ArenaMutator {
      inner: self,
      collector,
    }
  }

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    match delta {
      ArenaDelta::Mutate(_) => todo!(),
      ArenaDelta::Insert(_) => todo!(),
      ArenaDelta::Remove(_) => todo!(),
    }
  }

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    for (handle, item) in self {
      cb(ArenaDelta::Insert((item.clone(), handle)));
      item.expand(|d| cb(ArenaDelta::Mutate((d, handle))))
    }
  }
}

#[derive(Clone)]
pub enum ArenaDelta<T: IncrementAble> {
  Mutate((DeltaOf<T>, Handle<T>)),
  Insert((T, Handle<T>)),
  Remove(Handle<T>),
}

pub struct ArenaMutator<'a, T: IncrementAble + Clone> {
  inner: &'a mut Arena<T>,
  collector: &'a mut dyn FnMut(DeltaOf<Arena<T>>),
}

impl<'a, T: IncrementAble + Clone> MutatorApply<Arena<T>> for ArenaMutator<'a, T> {
  fn apply(&mut self, delta: DeltaOf<Arena<T>>) {
    self.inner.apply(delta).unwrap();
  }
}
