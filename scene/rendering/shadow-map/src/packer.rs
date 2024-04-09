use std::sync::Arc;

use parking_lot::RwLock;

use crate::*;

pub fn reactive_pack_2d_to_3d(
  mut config: ShadowSizeConfig,
  size: Box<dyn ReactiveCollection<u32, Size>>,
) -> (
  impl ReactiveCollection<u32, ShadowMapAddressInfo>,
  impl Stream<Item = SizeWithDepth> + Unpin,
) {
  config.make_sure_valid();

  let (sender, rev) = single_value_channel();

  let packer: PackerImpl = GrowablePacker::new(
    SizeWithDepth {
      depth: NonZeroU32::new(1).unwrap(),
      size: Size::from_u32_pair_min_one((512, 512)),
    },
    |config| {
      //
      None
    },
    |relocation| {
      //
    },
  );

  let packer = Packer {
    packer: Arc::new(RwLock::new(packer)),
    size_source: size,
  };

  (packer, rev)
}

type PackerImpl = GrowablePacker<MultiLayerTexturePacker<EtagerePacker>>;
struct Packer {
  packer: Arc<RwLock<PackerImpl>>,
  size_source: Box<dyn ReactiveCollection<u32, Size>>,
}

impl ReactiveCollection<u32, ShadowMapAddressInfo> for Packer {
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<u32, ShadowMapAddressInfo> {
    if let Poll::Ready(size_changes) = self.size_source.poll_changes(cx) {
      for (id, size) in size_changes.iter_key_value() {
        match size {
          ValueChange::Delta(_, _) => todo!(),
          ValueChange::Remove(_) => todo!(),
        }
      }
    }
    todo!()
  }

  fn access(&self) -> PollCollectionCurrent<u32, ShadowMapAddressInfo> {
    todo!()
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    // consider trigger packer shrink logic here?
    self.size_source.extra_request(request);
  }
}
