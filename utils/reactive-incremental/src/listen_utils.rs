use crate::*;

#[macro_export]
macro_rules! with_field {
  ($ty:ty =>$field:tt) => {
    |view, send| match view {
      incremental::MaybeDeltaRef::All(value) => send(value.$field.clone()),
      incremental::MaybeDeltaRef::Delta(delta) => {
        if let incremental::DeltaOf::<$ty>::$field(field) = delta {
          send(field.clone())
        }
      }
    }
  };
}

#[macro_export]
macro_rules! with_field_expand {
  ($ty:ty =>$field:tt) => {
    |view, send| match view {
      incremental::MaybeDeltaRef::All(value) => value.$field.expand(send),
      incremental::MaybeDeltaRef::Delta(delta) => {
        if let incremental::DeltaOf::<$ty>::$field(field) = delta {
          send(field.clone())
        }
      }
    }
  };
}

#[macro_export]
macro_rules! with_field_change {
  ($ty:ty =>$field:tt) => {
    |view, send| match view {
      incremental::MaybeDeltaRef::All(value) => send(()),
      incremental::MaybeDeltaRef::Delta(delta) => {
        if let incremental::DeltaOf::<$ty>::$field(field) = delta {
          send(())
        }
      }
    }
  };
}

pub fn all_delta<T: IncrementalBase>(view: MaybeDeltaRef<T>, send: &dyn Fn(T::Delta)) {
  all_delta_with(true, Some)(view, send)
}

pub fn all_delta_no_init<T: IncrementalBase>(view: MaybeDeltaRef<T>, send: &dyn Fn(T::Delta)) {
  all_delta_with(false, Some)(view, send)
}

pub fn any_change<T: IncrementalBase>(view: MaybeDeltaRef<T>, send: &dyn Fn(())) {
  any_change_with(true)(view, send)
}

pub fn any_change_no_init<T: IncrementalBase>(view: MaybeDeltaRef<T>, send: &dyn Fn(())) {
  any_change_with(false)(view, send)
}

pub fn no_change<T: IncrementalBase>(_view: MaybeDeltaRef<T>, _send: &dyn Fn(())) {
  // do nothing at all
}

#[inline(always)]
pub fn any_change_with<T: IncrementalBase>(
  should_send_when_init: bool,
) -> impl Fn(MaybeDeltaRef<T>, &dyn Fn(())) {
  move |view, send| match view {
    MaybeDeltaRef::All(_) => {
      if should_send_when_init {
        send(())
      }
    }
    MaybeDeltaRef::Delta(_) => send(()),
  }
}

#[inline(always)]
pub fn all_delta_with<T: IncrementalBase, X>(
  should_send_when_init: bool,
  filter_map: impl Fn(T::Delta) -> Option<X>,
) -> impl Fn(MaybeDeltaRef<T>, &dyn Fn(X)) {
  move |view, send| {
    let my_send = |d| {
      if let Some(d) = filter_map(d) {
        send(d)
      }
    };
    match view {
      MaybeDeltaRef::All(value) => {
        if should_send_when_init {
          value.expand(my_send)
        }
      }
      MaybeDeltaRef::Delta(delta) => my_send(delta.clone()),
    }
  }
}

pub trait IncrementalListenBy<T: IncrementalBase> {
  // todo remove box when compiler fixed
  fn listen_by<N, C, U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
    channel_builder: &mut C,
  ) -> Box<dyn Stream<Item = N> + Unpin>
  where
    U: Send + Sync + 'static,
    N: 'static,
    C: ChannelLike<U, Message = N>;

  fn unbound_listen_by<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> Box<dyn Stream<Item = U> + Unpin>
  where
    U: Send + Sync + 'static,
  {
    self.listen_by::<U, _, _>(mapper, &mut DefaultUnboundChannel)
  }

  fn single_listen_by<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> Box<dyn Stream<Item = U> + Unpin>
  where
    U: Send + Sync + 'static,
    U: IncrementalBase<Delta = U>,
  {
    self.listen_by::<U, _, _>(mapper, &mut DefaultSingleValueChannel)
  }

  // todo remove box when compiler fixed
  fn create_drop(&self) -> Box<dyn Future<Output = ()> + Unpin> {
    let mut s = self.single_listen_by(no_change);

    let r = Box::pin(async move {
      loop {
        if s.next().await.is_none() {
          break;
        }
      }
    });

    Box::new(r)
  }
}
