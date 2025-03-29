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
    all_size_sender: Arc::new(size_sender),
  };

  (allocator, size_rev)
}

type AllocationHandle = xalloc::tlsf::TlsfRegion<xalloc::arena::sys::Ptr>;

struct ReactiveAllocator<T> {
  source: T,
  allocator: Arc<RwLock<Allocator>>,
  all_size_sender: Arc<SingleSender<u32>>,
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

#[derive(Clone)]
struct AllocatorView {
  internal: LockReadGuardHolder<Allocator>,
}

impl Query for AllocatorView {
  type Key = u32;
  type Value = u32;
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
    self.internal.allocated.iter().map(|(k, v)| (*k, v.1))
  }

  fn access(&self, key: &u32) -> Option<u32> {
    self.internal.allocated.get(key).map(|v| v.1)
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

impl<T> AsyncQueryCompute for ReactiveAllocator<T>
where
  T: AsyncQueryCompute<Key = u32, Value = u32>,
{
  type Task = impl Future<Output = (Self::Changes, Self::View)> + 'static;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let allocator = self.allocator.clone();
    let all_size_sender = self.all_size_sender.clone();

    let source = self.source.create_task(cx);
    cx.then_spawn(source, |source, cx| {
      ReactiveAllocator {
        allocator,
        source,
        all_size_sender,
      }
      .resolve(cx)
    })
  }
}

impl<T: QueryCompute> QueryCompute for ReactiveAllocator<T>
where
  T: QueryCompute<Key = u32, Value = u32>,
{
  type Key = u32;
  type Value = u32;

  type Changes = Option<FastHashMap<u32, ValueChange<u32>>>;
  type View = AllocatorView;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let mut allocator = self.allocator.write();

    let (d, v) = self.source.resolve(cx);
    cx.keep_view_alive(v);

    let (sender, mut accumulated_mutations) = collective_channel::<u32, u32>();

    unsafe {
      sender.lock();

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
                sender.send(relocation.idx, delta);
              },
            ) {
              let delta = ValueChange::Delta(new_alloc, previous_offset);
              sender.send(idx, delta);
            } else if let Some(previous_offset) = previous_offset {
              sender.send(idx, ValueChange::Remove(previous_offset));
            }
          }
          ValueChange::Remove(_) => {
            let offset = allocator.dealloc(idx);
            sender.send(idx, ValueChange::Remove(offset));
          }
        }
      }

      sender.unlock();
    }

    drop(allocator);

    noop_ctx!(cx);
    let mut d = None;
    if let Poll::Ready(Some(r)) = accumulated_mutations.poll_next_unpin(cx) {
      d = Some(r);
    }

    let v = self.allocator.make_read_holder();

    (d, AllocatorView { internal: v })
  }
}

impl<T> ReactiveQuery for ReactiveAllocator<T>
where
  T: ReactiveQuery<Key = u32, Value = u32>,
{
  type Key = u32;
  type Value = u32;
  type Compute = ReactiveAllocator<T::Compute>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    let source = self.source.describe(cx);

    ReactiveAllocator {
      allocator: self.allocator.clone(),
      source,
      all_size_sender: self.all_size_sender.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.source.request(request)
  }
}
