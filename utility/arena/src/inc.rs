use crate::*;

impl<T: IncrementalBase + Clone + Send + Sync> IncrementalBase for Arena<T> {
  type Delta = ArenaDelta<T>;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    for (handle, item) in self {
      cb(ArenaDelta::Insert((item.clone(), handle)));
    }
  }
}

#[derive(Debug)]
pub enum ArenaMutationFailure<T> {
  AccessFailed,
  InputHandleNotMatchInsertResult,
  Inner(T),
}

impl<T: ApplicableIncremental + Clone + Send + Sync> ApplicableIncremental for Arena<T> {
  type Error = ArenaMutationFailure<T::Error>;

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    match delta {
      ArenaDelta::Mutate((delta, handle)) => {
        let item = self
          .get_mut(handle)
          .ok_or(ArenaMutationFailure::AccessFailed)?;
        item.apply(delta).map_err(ArenaMutationFailure::Inner)
      }
      ArenaDelta::Insert((item, handle)) => {
        let r_handle = self.insert(item);
        (handle == r_handle)
          .then_some(())
          .ok_or(ArenaMutationFailure::InputHandleNotMatchInsertResult)
      }
      ArenaDelta::Remove(handle) => self
        .remove(handle)
        .map(|_| {})
        .ok_or(ArenaMutationFailure::AccessFailed),
    }
  }
}

/// arena's delta contains the inner state of arena(the handle)
/// it's hard or impossible for outside to construct the delta beforehand to express the mutation
#[derive(Clone)]
pub enum ArenaDelta<T: IncrementalBase> {
  Mutate((DeltaOf<T>, Handle<T>)),
  Insert((T, Handle<T>)),
  Remove(Handle<T>),
}
