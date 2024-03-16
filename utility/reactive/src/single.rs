use crate::*;

pub struct IncrementalSignal<T: IncrementalBase> {
  guid: u64,
  inner: T,
  pub delta_source: EventSource<T::Delta>,
  _counting: Counted<Self>,
}

impl<T: IncrementalBase> AsRef<T> for IncrementalSignal<T> {
  fn as_ref(&self) -> &T {
    &self.inner
  }
}

impl<T: IncrementalBase> From<T> for IncrementalSignal<T> {
  fn from(inner: T) -> Self {
    Self::new(inner)
  }
}

impl<T: IncrementalBase> GlobalIdentified for IncrementalSignal<T> {
  fn guid(&self) -> u64 {
    self.guid
  }
}
impl<T: IncrementalBase> AsRef<dyn GlobalIdentified> for IncrementalSignal<T> {
  fn as_ref(&self) -> &(dyn GlobalIdentified + 'static) {
    self
  }
}
impl<T: IncrementalBase> AsMut<dyn GlobalIdentified> for IncrementalSignal<T> {
  fn as_mut(&mut self) -> &mut (dyn GlobalIdentified + 'static) {
    self
  }
}

impl<T: IncrementalBase> IncrementalSignal<T> {
  pub fn new(inner: T) -> Self {
    Self {
      inner,
      guid: alloc_global_res_id(),
      delta_source: Default::default(),
      _counting: Default::default(),
    }
  }

  pub fn mutate_unchecked<R>(&mut self, mutator: impl FnOnce(&mut T) -> R) -> R {
    mutator(&mut self.inner)
  }

  pub fn mutate<R>(&mut self, mutator: impl FnOnce(Mutating<T>) -> R) -> R {
    let data = &mut self.inner;
    let dispatcher = &self.delta_source;
    mutator(Mutating {
      inner: data,
      collector: &mut |delta, _| {
        dispatcher.emit(delta);
      },
    })
  }
}

impl<T: IncrementalBase> IncrementalListenBy<T> for IncrementalSignal<T> {
  fn listen_by<N, C, U>(
    &self,
    mut mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
    channel_builder: &mut C,
  ) -> Box<dyn Stream<Item = N> + Unpin>
  where
    U: Send + Sync + 'static,
    C: ChannelLike<U, Message = N>,
  {
    let (sender, receiver) = channel_builder.build();

    mapper(MaybeDeltaRef::All(self), &|mapped| {
      C::send(&sender, mapped);
    });

    let remove_token = self.delta_source.on(move |v| {
      mapper(MaybeDeltaRef::Delta(v), &|mapped| {
        C::send(&sender, mapped);
      });
      C::is_closed(&sender)
    });

    let dropper = EventSourceDropper::new(remove_token, self.delta_source.make_weak());
    Box::new(DropperAttachedStream::new(dropper, receiver))
  }
}

impl<T: Default + IncrementalBase> Default for IncrementalSignal<T> {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl<T: IncrementalBase> std::ops::Deref for IncrementalSignal<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}
