use futures::Stream;
use reactive_stream::{noop_ctx, single_value_channel, SingleSender};

use crate::*;

/// input sizes, return allocation offsets and size all
pub fn reactive_linear_allocation(
  init_count: u32,
  max_count: u32,
  input: impl ReactiveQuery<Key = u32, Value = u32>,
) -> (
  impl ReactiveQuery<Key = u32, Value = u32>,
  impl Stream<Item = u32>,
) {
  assert!(init_count <= max_count);

  let (size_sender, size_rev) = single_value_channel();

  let allocator = xalloc::SysTlsf::new(init_count);
  let allocator = Allocator {
    allocator,
    max_count,
    current_count: init_count,
    allocated: Default::default(),
  };

  let allocator = ReactiveAllocator {
    source: input,
    allocator: Arc::new(RwLock::new(allocator)),
    all_size_sender: size_sender,
  };

  (allocator, size_rev)
}

type AllocationHandle = xalloc::tlsf::TlsfRegion<xalloc::arena::sys::Ptr>;

struct ReactiveAllocator<T> {
  source: T,
  allocator: Arc<RwLock<Allocator>>,
  all_size_sender: SingleSender<u32>,
}

struct Allocator {
  // todo should we try other allocator that support relocate and shrink??
  //
  // In the rust ecosystem, there are many allocator implementations but it's rare to find one for
  // our use case, because what we want is an allocator to manage the external memory not the
  // internal, which means the allocate does not own the memory and is unable to store internal
  // allocation states and data structures into the requested but not allocated memory space.
  allocator: xalloc::SysTlsf<u32>,
  max_count: u32,
  current_count: u32,
  allocated: FastHashMap<u32, (AllocationHandle, u32, u32)>,
}

impl Query for LockReadGuardHolder<Allocator> {
  type Key = u32;
  type Value = u32;
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
    self.allocated.iter().map(|(k, v)| (*k, v.1))
  }

  fn access(&self, key: &u32) -> Option<u32> {
    self.allocated.get(key).map(|v| v.1)
  }
}

struct AllocationRelocation {
  idx: u32,
  previous: u32,
  new: u32,
}

impl Allocator {
  pub fn alloc_and_check_grow(
    &mut self,
    idx: u32,
    size: u32,
    on_grow: impl Fn(u32),
    on_relocation: impl Fn(AllocationRelocation),
  ) -> Option<u32> {
    loop {
      if let Some(r) = self.allocator.alloc(size) {
        let offset = r.1;
        self.allocated.insert(idx, (r.0, r.1, size));
        return Some(offset);
      } else if self.current_count < self.max_count {
        // todo, should we expose the current allocation info to avoid loop grow?
        // todo, we should support batch allocation to further avoid loop grow!
        let new_count = (self.current_count * 2).min(self.max_count);
        self.allocator = xalloc::SysTlsf::new(new_count);
        self.current_count = new_count;
        on_grow(new_count);

        // do reallocate previous all allocated
        let previous = std::mem::take(&mut self.allocated);
        for (id, (_, previous_offset, size)) in previous {
          let new = self
            .allocator
            .alloc(size)
            .expect("allocator grow relocation must success");
          on_relocation(AllocationRelocation {
            idx: id,
            previous: previous_offset,
            new: new.1,
          });
          self.allocated.insert(id, (new.0, new.1, size));
        }
      } else {
        return None;
      }
    }
  }

  pub fn dealloc(&mut self, idx: u32) -> u32 {
    let (handle, offset, _) = self.allocated.remove(&idx).unwrap();
    self.allocator.dealloc(handle).unwrap();
    offset
  }
}

struct ReactiveAllocatorCompute<T> {
  allocator: Option<LockWriteGuardHolder<Allocator>>,
  source: T,
  all_size_sender: SingleSender<u32>,
  sender: CollectiveMutationSender<u32, u32>,
  accumulated_mutations: CollectiveMutationReceiver<u32, u32>,
}

impl<T: QueryCompute> QueryCompute for ReactiveAllocatorCompute<T>
where
  T: QueryCompute<Key = u32, Value = u32>,
{
  type Key = u32;
  type Value = u32;

  type Changes = impl Query<Key = u32, Value = ValueChange<u32>>;
  type View = impl Query<Key = u32, Value = u32>;

  fn resolve(&mut self) -> (Self::Changes, Self::View) {
    let mut allocator = self.allocator.take().unwrap();

    let (d, _) = self.source.resolve();

    unsafe {
      self.sender.lock();

      for (idx, size_change) in d.iter_key_value() {
        match size_change {
          ValueChange::Delta(new_size, previous_size) => {
            let previous_offset = previous_size.map(|_| allocator.dealloc(idx));

            if let Some(new_alloc) = allocator.alloc_and_check_grow(
              idx,
              new_size,
              |new_size| {
                self.all_size_sender.update(new_size).ok();
              },
              |relocation| {
                let delta = ValueChange::Delta(relocation.new, relocation.previous.into());
                self.sender.send(relocation.idx, delta);
              },
            ) {
              let delta = ValueChange::Delta(new_alloc, previous_offset);
              self.sender.send(idx, delta);
            } else if let Some(previous_offset) = previous_offset {
              self.sender.send(idx, ValueChange::Remove(previous_offset));
            }
          }
          ValueChange::Remove(_) => {
            let offset = allocator.dealloc(idx);
            self.sender.send(idx, ValueChange::Remove(offset));
          }
        }
      }

      self.sender.unlock();
    }

    noop_ctx!(cx);
    let d = if let Poll::Ready(Some(r)) = self.accumulated_mutations.poll_impl(cx) {
      r
    } else {
      QueryExt::into_boxed(EmptyQuery::default())
    };

    let v = allocator.downgrade_to_read();

    (d, v)
  }
}

impl<T> ReactiveQuery for ReactiveAllocator<T>
where
  T: ReactiveQuery<Key = u32, Value = u32>,
{
  type Key = u32;
  type Value = u32;
  type Compute = ReactiveAllocatorCompute<T::Compute>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    let source = self.source.describe(cx);

    let (sender, accumulated_mutations) = collective_channel::<u32, u32>();

    ReactiveAllocatorCompute {
      allocator: Some(self.allocator.make_write_holder()),
      source,
      all_size_sender: self.all_size_sender.clone(),
      sender,
      accumulated_mutations,
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.source.request(request)
  }
}
