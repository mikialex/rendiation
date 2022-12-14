use crate::*;

impl<T: IncrementalBase + Clone + Send + Sync> IncrementalBase for Arena<T> {
  type Delta = ArenaDelta<T>;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    for (handle, item) in self {
      cb(ArenaDelta::Insert((item.clone(), handle)));
      item.expand(|d| cb(ArenaDelta::Mutate((d, handle))))
    }
  }
}

impl<T> IncrementalMutatorHelper for Arena<T>
where
  Self: IncrementalBase,
  T: IncrementalBase + Clone,
{
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
}

#[derive(Clone)]
pub enum ArenaDelta<T: IncrementalBase> {
  Mutate((DeltaOf<T>, Handle<T>)),
  Insert((T, Handle<T>)),
  Remove(Handle<T>),
}

pub struct ArenaMutator<'a, T: IncrementalBase + Clone + Send + Sync> {
  inner: &'a mut Arena<T>,
  collector: &'a mut dyn FnMut(DeltaOf<Arena<T>>),
}

impl<'a, T: IncrementalBase + Clone + Send + Sync> ArenaMutator<'a, T> {
  pub fn insert(&mut self, item: T) -> Handle<T> {
    let handle = self.inner.insert(item.clone());
    (self.collector)(ArenaDelta::Insert((item, handle)));
    handle
  }
}
