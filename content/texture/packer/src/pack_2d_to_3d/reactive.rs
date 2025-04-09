use rendiation_texture_core::SizeWithDepth;

use self::{
  growable::{GrowablePacker, PackResultRelocation},
  pack_impl::etagere_wrap::EtagerePacker,
};
use super::*;

pub fn reactive_pack_2d_to_3d(
  mut config: MultiLayerTexturePackerConfig,
  size: BoxedDynReactiveQuery<u32, Size>,
) -> (
  impl ReactiveQuery<Key = u32, Value = PackResult2dWithDepth>,
  impl Stream<Item = SizeWithDepth> + Unpin,
) {
  config.make_sure_valid();

  let (size_sender, size_rev) = single_value_channel();
  size_sender.update(config.init_size).unwrap();

  let packer: PackerImpl = GrowablePacker::new(config.init_size);

  let packer = Packer {
    max_size: config.max_size,
    packer: Arc::new(RwLock::new(packer)),
    size_source: size,
    mapping: Default::default(),
    rev_mapping: Default::default(),
    all_size_sender: Arc::new(size_sender),
  };

  (packer, size_rev)
}

type PackerImpl = GrowablePacker<MultiLayerTexturePacker<EtagerePacker>>;

struct Packer {
  max_size: SizeWithDepth,
  size_source: BoxedDynReactiveQuery<u32, Size>,

  packer: Arc<RwLock<PackerImpl>>,
  // todo, i think this is not necessary if the packer lib not generate id
  mapping: Arc<RwLock<FastHashMap<PackId, u32>>>,
  rev_mapping: Arc<RwLock<FastHashMap<u32, PackId>>>,

  all_size_sender: Arc<SingleSender<SizeWithDepth>>,
}

#[derive(Clone)]
struct PackerCurrentView {
  rev_mapping: LockReadGuardHolder<FastHashMap<u32, PackId>>,
  packer: LockReadGuardHolder<PackerImpl>,
}

impl Query for PackerCurrentView {
  type Key = u32;
  type Value = PackResult2dWithDepth;
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, PackResult2dWithDepth)> + '_ {
    self
      .rev_mapping
      .iter()
      .map(|(k, v)| (*k, self.packer.current_states().1.get(v).unwrap().1))
  }

  fn access(&self, key: &u32) -> Option<PackResult2dWithDepth> {
    let pack_id = self.rev_mapping.get(key)?;
    let result = self.packer.current_states().1.get(pack_id)?.1;
    Some(result)
  }
}

impl ReactiveQuery for Packer {
  type Key = u32;
  type Value = PackResult2dWithDepth;

  type Compute = PackerCompute<BoxedDynQueryCompute<u32, Size>>;
  fn describe(&self, cx: &mut Context) -> Self::Compute {
    PackerCompute {
      size_source: self.size_source.describe_dyn(cx),
      max_size: self.max_size,
      packer: self.packer.clone(),
      mapping: self.mapping.clone(),
      rev_mapping: self.rev_mapping.clone(),
      all_size_sender: self.all_size_sender.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    // consider trigger packer shrink logic here?
    self.size_source.request(request);
  }
}

struct PackerCompute<T> {
  size_source: T,
  max_size: SizeWithDepth,

  packer: Arc<RwLock<PackerImpl>>,
  // todo, i think this is not necessary if the packer lib not generate id
  mapping: Arc<RwLock<FastHashMap<PackId, u32>>>,
  rev_mapping: Arc<RwLock<FastHashMap<u32, PackId>>>,
  all_size_sender: Arc<SingleSender<SizeWithDepth>>,
}

impl<T: AsyncQueryCompute<Key = u32, Value = Size>> AsyncQueryCompute for PackerCompute<T> {
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let max_size = self.max_size;
    let packer = self.packer.clone();
    let mapping = self.mapping.clone();
    let rev_mapping = self.rev_mapping.clone();
    let all_size_sender = self.all_size_sender.clone();
    let size_source = self.size_source.create_task(cx);
    cx.then_spawn_compute(size_source, move |size_source| PackerCompute {
      size_source,
      max_size,
      packer,
      mapping,
      rev_mapping,
      all_size_sender,
    })
    .into_boxed_future()
  }
}

impl<T: QueryCompute<Key = u32, Value = Size>> QueryCompute for PackerCompute<T> {
  type Key = u32;
  type Value = PackResult2dWithDepth;

  type Changes = Option<FastHashMap<u32, ValueChange<PackResult2dWithDepth>>>;
  type View = PackerCurrentView;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (d, v) = self.size_source.resolve(cx);
    cx.keep_view_alive(v);
    let (sender, mut rev) = collective_channel::<u32, PackResult2dWithDepth>();

    {
      unsafe {
        sender.lock();
        let mut mapping = self.mapping.write();
        let mut rev_mapping = self.rev_mapping.write();
        let packer = &mut self.packer.write();

        let mut grow = |config: SizeWithDepth| {
          let max = self.max_size;
          let width_capacity = max.size.width_usize() - config.size.width_usize();
          let height_capacity = max.size.height_usize() - config.size.height_usize();
          let depth_capacity = u32::from(max.depth) - u32::from(config.depth);

          if depth_capacity == 0 && height_capacity == 0 && width_capacity == 0 {
            return None;
          }

          let target_config = if height_capacity == 0 && width_capacity == 0 {
            // when we only have depth space available, increase depth only one step each grow
            SizeWithDepth {
              depth: NonZeroU32::new(u32::from(config.depth) + 1).unwrap(),
              size: config.size,
            }
          } else {
            let width_target = (config.size.width_usize() * 2).min(max.size.width_usize());
            let height_target = (config.size.width_usize() * 2).min(max.size.width_usize());
            SizeWithDepth {
              depth: config.depth,
              size: Size::from_usize_pair_min_one((width_target, height_target)),
            }
          };

          self.all_size_sender.update(target_config).ok();

          Some(target_config)
        };

        for (id, size) in d.iter_key_value() {
          match size {
            ValueChange::Delta(new_size, _) => {
              if let Some(pack_id) = rev_mapping.remove(&id) {
                mapping.remove(&pack_id);
                let previous = packer.unpack(pack_id).unwrap();
                let delta = ValueChange::Remove(previous);
                sender.send(id, delta);
              }

              let mut relocate = |relocation: PackResultRelocation<PackResult2dWithDepth>| {
                let idx = mapping.remove(&relocation.previous.id).unwrap();
                let previous = relocation.previous.result;
                sender.send(idx, ValueChange::Remove(previous));

                mapping.insert(relocation.new.id, idx);
                let current = relocation.new.result;
                sender.send(idx, ValueChange::Delta(current, None));

                rev_mapping.insert(idx, relocation.new.id);
              };

              let pack_result = packer.pack_and_check_grow(new_size, &mut grow, &mut relocate);

              if let Ok(pack_result) = pack_result {
                rev_mapping.insert(id, pack_result.id);
                mapping.insert(pack_result.id, id);
                let delta = ValueChange::Delta(pack_result.result, None);

                sender.send(id, delta);
              }
            }
            ValueChange::Remove(_) => {
              let pack_id = rev_mapping.remove(&id).unwrap();
              mapping.remove(&pack_id);
              let previous = packer.unpack(pack_id).unwrap();
              let delta = ValueChange::Remove(previous);
              sender.send(id, delta);
            }
          }
        }

        sender.unlock();
      }
    }

    let v = PackerCurrentView {
      rev_mapping: self.rev_mapping.make_read_holder(),
      packer: self.packer.make_read_holder(),
    };

    noop_ctx!(cx);
    let mut d = None;
    if let Poll::Ready(Some(r)) = rev.poll_next_unpin(cx) {
      d = Some(r);
    }

    (d, v)
  }
}
