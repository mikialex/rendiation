use crate::*;
use etagere::{size2, AllocId, AtlasAllocator};

pub struct EtagerePacker {
  config: PackerConfig,
  inner: AtlasAllocator,
}

impl EtagerePacker {
  pub fn new(config: PackerConfig) -> Self {
    let (width, height) = config.init_size.into_usize();
    let inner = AtlasAllocator::new(size2(width as i32, height as i32));
    Self { config, inner }
  }
}

impl Default for EtagerePacker {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl BaseTexturePacker for EtagerePacker {
  fn config(&mut self, config: PackerConfig) {
    self.config = config;
    self.reset();
  }

  fn reset(&mut self) {
    *self = Self::new(self.config)
  }
}

impl RePackablePacker for EtagerePacker {
  fn pack_with_id(&mut self, input: Size) -> Result<PackResultWithId, PackError> {
    let result = self
      .inner
      .allocate(size2(
        input.width_usize() as i32,
        input.height_usize() as i32,
      ))
      .ok_or(PackError::SpaceNotEnough)?;

    Ok(PackResultWithId {
      result: PackResult {
        range: TextureRange {
          origin: (
            result.rectangle.min.x as usize,
            result.rectangle.min.y as usize,
          )
            .into(),
          size: input,
        },
        rotated: false,
      },
      id: PackId(result.id.serialize()),
    })
  }

  fn unpack(&mut self, id: PackId) -> Result<(), UnpackError> {
    let id = AllocId::deserialize(id.0);
    self.inner.deallocate(id);
    Ok(())
  }
}
