use crate::*;

pub struct MultiAccessGPUDataBuilder {
  source: Box<dyn DynReactiveOneToManyRelation<One = u32, Many = u32>>,
  allocator: StorageBufferRangeAllocatePool<u32>,
  meta: CommonStorageBufferImplWithHostBackup<GPURangeInfo>,
}

pub struct MultiAccessGPUDataBuilderInit {
  pub max_possible_many_count: u32,
  pub max_possible_one_count: u32,
  pub init_many_count_capacity: u32,
  pub init_one_count_capacity: u32,
}

impl MultiAccessGPUDataBuilder {
  pub fn new(
    gpu: &GPU,
    source: impl ReactiveOneToManyRelation<One = u32, Many = u32>,
    init: MultiAccessGPUDataBuilderInit,
  ) -> Self {
    Self {
      source: Box::new(source),
      allocator: create_storage_buffer_range_allocate_pool(
        gpu,
        init.init_many_count_capacity,
        init.max_possible_many_count,
      ),
      meta: create_common_storage_buffer_with_host_backup_container(
        init.init_one_count_capacity,
        init.max_possible_one_count,
        gpu,
      ),
    }
  }
}

impl ReactiveGeneralQuery for MultiAccessGPUDataBuilder {
  type Output = MultiAccessGPUData;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let (changes, multi_access) = self.source.describe_with_inv_dyn(cx).resolve_kept();

    // collect all changed one.
    // for simplicity, we do full update for each "one"s data
    let mut dirtied_one = FastHashSet::default();
    for (_, change) in changes.iter_key_value() {
      match change {
        ValueChange::Delta(n, p) => {
          dirtied_one.insert(n);
          if let Some(p) = p {
            dirtied_one.insert(p);
          }
        }
        ValueChange::Remove(one) => {
          dirtied_one.insert(one);
        }
      }
    }

    // do all cleanup first to release empty space
    for changed_one in dirtied_one.iter() {
      let meta = self.meta.get(*changed_one).unwrap();
      if meta.start != u32::MAX {
        self.allocator.deallocate(meta.start);
      }

      self
        .meta
        .set_value(*changed_one, Default::default())
        .unwrap();
    }

    // write new data
    for changed_one in dirtied_one.iter() {
      if let Some(many_iter) = multi_access.access_multi(changed_one) {
        let many_idx = many_iter.collect::<Vec<_>>();
        let offset = self
          .allocator
          .allocate_values(&many_idx, &mut |relocation| {
            let mut meta = *self.meta.get(*changed_one).unwrap();
            meta.start = relocation.new_offset;
            self.meta.set_value(*changed_one, meta).unwrap();
          })
          .unwrap();

        let mut meta = *self.meta.get(*changed_one).unwrap();
        meta.start = offset;
        meta.len = many_idx.len() as u32;
        self.meta.set_value(*changed_one, meta).unwrap();
      }
    }

    MultiAccessGPUData {
      meta: self.meta.gpu().clone(),
      indices: self.allocator.gpu().clone(),
    }
  }
}

#[derive(Clone)]
pub struct MultiAccessGPUData {
  meta: StorageBufferReadonlyDataView<[GPURangeInfo]>,
  indices: StorageBufferReadonlyDataView<[u32]>,
}

impl MultiAccessGPUData {
  pub fn build(&self, cx: &mut ShaderBindGroupBuilder) -> MultiAccessGPUInvocation {
    MultiAccessGPUInvocation {
      meta: cx.bind_by(&self.meta),
      indices: cx.bind_by(&self.indices),
    }
  }

  pub fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.meta);
    cx.bind(&self.indices);
  }
}

pub trait MultiAccessGPUQueryResultCtxExt {
  fn take_multi_access_gpu(&mut self, token: QueryToken) -> Option<MultiAccessGPUData>;
}

impl MultiAccessGPUQueryResultCtxExt for QueryResultCtx {
  fn take_multi_access_gpu(&mut self, token: QueryToken) -> Option<MultiAccessGPUData> {
    self
      .take_result(token)?
      .downcast::<MultiAccessGPUData>()
      .map(|v| *v)
      .ok()
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, PartialEq)]
pub struct GPURangeInfo {
  pub start: u32,
  pub len: u32,
}

impl Default for GPURangeInfo {
  fn default() -> Self {
    Self {
      start: u32::MAX, // we use max as empty hint
      len: Default::default(),
      ..Zeroable::zeroed()
    }
  }
}

#[derive(Clone)]
pub struct MultiAccessGPUInvocation {
  pub meta: ShaderReadonlyPtrOf<[GPURangeInfo]>,
  pub indices: ShaderReadonlyPtrOf<[u32]>,
}

impl MultiAccessGPUInvocation {
  pub fn iter_refed_many_of(&self, one: Node<u32>) -> impl ShaderIterator<Item = Node<u32>> {
    MultiAccessGPUIter {
      indices: self.indices.clone(),
      meta: self.meta.index(one).load().expand(),
      cursor: val(0_u32).make_local_var(),
    }
  }
}

struct MultiAccessGPUIter {
  indices: ShaderReadonlyPtrOf<[u32]>,
  meta: ENode<GPURangeInfo>,
  cursor: ShaderPtrOf<u32>,
}

impl ShaderIterator for MultiAccessGPUIter {
  type Item = Node<u32>;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let current_next = self.cursor.load();
    self.cursor.store(current_next + val(1));
    let has_next = current_next.less_than(self.meta.len);
    let data = self
      .indices
      .index(current_next.min(self.meta.len - val(1)) + self.meta.start)
      .load();
    (has_next, data)
  }
}
