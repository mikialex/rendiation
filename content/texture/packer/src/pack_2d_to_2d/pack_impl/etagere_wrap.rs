use etagere::{size2, AllocId, AtlasAllocator};

use super::super::*;

pub struct EtagerePacker {
  inner: AtlasAllocator,
}

impl EtagerePacker {
  pub fn new(config: PackerConfig2d) -> Self {
    let (width, height) = config.full_size.into_usize();
    let inner = AtlasAllocator::new(size2(width as i32, height as i32));
    Self { inner }
  }
}

impl Default for EtagerePacker {
  fn default() -> Self {
    Self::new(Default::default())
  }
}

impl TexturePackerInit for EtagerePacker {
  type Config = PackerConfig2d;

  fn init_by_config(config: Self::Config) -> Self {
    Self::new(config)
  }
}

impl RePackablePacker for EtagerePacker {
  type Input = Size;
  type PackOutput = PackResult2d;

  fn pack_with_id(&mut self, input: Size) -> Result<PackResultWithId<PackResult2d>, PackError> {
    let result = self
      .inner
      .allocate(size2(
        input.width_usize() as i32,
        input.height_usize() as i32,
      ))
      .ok_or(PackError::SpaceNotEnough)?;

    Ok(PackResultWithId {
      result: PackResult2d {
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
