use rendiation_texture_core::SizeWithDepth;

use self::{
  growable::{GrowablePacker, PackResultRelocation},
  pack_impl::etagere_wrap::EtagerePacker,
};
use super::*;

pub fn reactive_pack_2d_to_3d(
  mut config: MultiLayerTexturePackerConfig,
  size: Box<dyn DynReactiveCollection<u32, Size>>,
) -> (
  impl ReactiveCollection<u32, PackResult2dWithDepth>,
  impl Stream<Item = SizeWithDepth> + Unpin,
) {
  config.make_sure_valid();

  let (size_sender, size_rev) = single_value_channel();
  let (sender, rev) = collective_channel::<u32, PackResult2dWithDepth>();

  let packer: PackerImpl = GrowablePacker::new(config.init_size);

  let packer = Packer {
    max_size: config.max_size,
    packer: Arc::new(RwLock::new(packer)),
    size_source: size,
    mapping: Default::default(),
    rev_mapping: Default::default(),
    accumulated_mutations: rev,
    sender,
    all_size_sender: size_sender,
  };

  (packer, size_rev)
}

type PackerImpl = GrowablePacker<MultiLayerTexturePacker<EtagerePacker>>;

struct Packer {
  max_size: SizeWithDepth,
  size_source: Box<dyn DynReactiveCollection<u32, Size>>,

  packer: Arc<RwLock<PackerImpl>>,
  // todo, i think this is not necessary if the packer lib not generate id
  mapping: Arc<RwLock<FastHashMap<PackId, u32>>>,
  rev_mapping: Arc<RwLock<FastHashMap<u32, PackId>>>,

  sender: CollectiveMutationSender<u32, PackResult2dWithDepth>,
  accumulated_mutations: CollectiveMutationReceiver<u32, PackResult2dWithDepth>,
  all_size_sender: SingleSender<SizeWithDepth>,
}

#[derive(Clone)]
struct PackerCurrentView {
  rev_mapping: LockReadGuardHolder<FastHashMap<u32, PackId>>,
  packer: LockReadGuardHolder<PackerImpl>,
}

impl VirtualCollection<u32, PackResult2dWithDepth> for PackerCurrentView {
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, PackResult2dWithDepth)> + '_ {
    self
      .packer
      .current_states()
      .1
      .iter()
      .map(|(k, v)| (k.0, v.1))
  }

  fn access(&self, key: &u32) -> Option<PackResult2dWithDepth> {
    let pack_id = self.rev_mapping.get(key)?;
    let result = self.packer.current_states().1.get(pack_id)?.1;
    Some(result)
  }
}

impl ReactiveCollection<u32, PackResult2dWithDepth> for Packer {
  type Changes = Box<dyn DynVirtualCollection<u32, ValueChange<PackResult2dWithDepth>>>;
  type View = PackerCurrentView;
  type Task = impl Future<Output = (Self::Changes, Self::View)>;
  fn poll_changes(&self, cx: &mut Context) -> Self::Task {
    let task = self.size_source.poll_changes(cx);
    let mapping = self.mapping.clone();
    let rev_mapping = self.rev_mapping.clone();
    let sender = self.sender.clone();
    let packer = self.packer.clone();
    let all_size_sender = self.all_size_sender.clone();
    let mut accumulated_mutations = self.accumulated_mutations.clone();
    let max = self.max_size;

    async move {
      let (d, _) = task.await;
      {
        unsafe {
          sender.lock();
          let mut mapping = mapping.write();
          let mut rev_mapping = rev_mapping.write();
          let packer = &mut packer.write();

          let mut grow = |config: SizeWithDepth| {
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

            all_size_sender.update(target_config).ok();

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
        rev_mapping: rev_mapping.make_read_holder(),
        packer: packer.make_read_holder(),
      };

      let d = accumulated_mutations.next().await.unwrap_or(Box::new(()));

      (d, v)
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    // consider trigger packer shrink logic here?
    self.size_source.extra_request(request);
  }
}
