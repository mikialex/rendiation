use crate::*;

impl<T: IncrementalBase + Clone> IncrementalSignalStorage<T> {
  pub fn listen_by<N, C, U>(
    &self,
    mut mapper: impl FnMut(&StorageGroupChange<T>, &dyn Fn(U)) + Send + Sync + 'static,
    channel_builder: &mut C,
  ) -> impl Stream<Item = N> + Unpin
  where
    U: Send + Sync + 'static,
    C: ChannelLike<U, Message = N>,
  {
    let (sender, receiver) = channel_builder.build();

    {
      let data = self.inner.data.write().unwrap();

      for (index, data) in data.iter() {
        mapper(
          &StorageGroupChange::Create {
            data: unsafe { std::mem::transmute(data) },
            index,
          },
          &|mapped| {
            C::send(&sender, mapped);
          },
        )
      }
    }

    // could we try another way to do workaround this??
    let s: &'static Self = unsafe { std::mem::transmute(self) };

    let remove_token = s.on(move |v| {
      mapper(v, &|mapped| {
        C::send(&sender, mapped);
      });
      C::is_closed(&sender)
    });

    let dropper = EventSourceDropper::new(remove_token, self.inner.group_watchers.make_weak());
    DropperAttachedStream::new(dropper, receiver)
  }
}

// pub struct GroupSingleValue<T> {
//   deduplicate: FastHashMap<u32, T>,
// }

// impl<T> ChannelLike<T> for GroupSingleValue<T> {
//   type Message = Vec<T>;

//   type Sender;

//   type Receiver;

//   fn build(&mut self) -> (Self::Sender, Self::Receiver) {
//     todo!()
//   }

//   fn send(sender: &Self::Sender, message: T) -> bool {
//     todo!()
//   }

//   fn is_closed(sender: &Self::Sender) -> bool {
//     todo!()
//   }
// }
