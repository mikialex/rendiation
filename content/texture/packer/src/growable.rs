use crate::*;

pub struct GrowablePacker<P: RePackablePacker + TexturePackerInit> {
  packer: P,
  current_config: P::Config,
  result: FastHashMap<PackId, (P::Input, P::PackOutput)>,
}

impl<P: RePackablePacker + TexturePackerInit> GrowablePacker<P> {
  pub fn new(init: P::Config) -> Self {
    Self {
      packer: P::init_by_config(init.clone()),
      current_config: init,
      result: Default::default(),
    }
  }

  pub fn current_states(&self) -> (&P::Config, &FastHashMap<PackId, (P::Input, P::PackOutput)>) {
    (&self.current_config, &self.result)
  }

  pub fn pack_and_check_grow(
    &mut self,
    input: P::Input,
    on_grow: &mut impl FnMut(P::Config) -> Option<P::Config>,
    relocation_callback: &mut impl FnMut(PackResultRelocation<P::PackOutput>),
  ) -> Result<PackResultWithId<P::PackOutput>, PackError> {
    loop {
      if let Ok(r) = self.packer.pack_with_id(input.clone()) {
        self.result.insert(r.id, (input.clone(), r.result.clone()));
        return Ok(r);
      } else if let Some(new_config) = on_grow(self.current_config.clone()) {
        // todo, should we expose the current allocation info to avoid loop grow?
        // todo, we should support batch allocation to further avoid loop grow!
        self.packer = P::init_by_config(new_config.clone());
        self.current_config = new_config;

        // do repack previous all packed
        let previous = std::mem::take(&mut self.result);
        for (id, (input, result)) in previous {
          let new_result = self
            .packer
            .pack_with_id(input.clone())
            .expect("pack after grow must success");

          self
            .result
            .insert(new_result.id, (input, new_result.result.clone()));

          let previous = PackResultWithId { result, id };
          let new = new_result;
          relocation_callback(PackResultRelocation { previous, new });
        }
      } else {
        return Err(PackError::SpaceNotEnough);
      }
    }
  }

  pub fn unpack(&mut self, id: PackId) -> Result<P::PackOutput, UnpackError> {
    self.packer.unpack(id)?;
    let r = self.result.remove(&id).unwrap();
    Ok(r.1)
  }
}

pub struct PackResultRelocation<T> {
  pub previous: PackResultWithId<T>,
  pub new: PackResultWithId<T>,
}

/// for performance reason this packer is not recommended to use
pub struct GrowableSelfContainedPacker<P: RePackablePacker + TexturePackerInit> {
  packer: GrowablePacker<P>,
  on_grow: Box<dyn FnMut(P::Config) -> Option<P::Config> + Send + Sync>,
  relocation_callback: Box<dyn FnMut(PackResultRelocation<P::PackOutput>) + Send + Sync>,
}

impl<P: RePackablePacker + TexturePackerInit> GrowableSelfContainedPacker<P> {
  pub fn new(
    init: P::Config,
    grow: impl FnMut(P::Config) -> Option<P::Config> + 'static + Send + Sync,
    relocation: impl FnMut(PackResultRelocation<P::PackOutput>) + 'static + Send + Sync,
  ) -> Self {
    Self {
      packer: GrowablePacker::new(init),
      on_grow: Box::new(grow),
      relocation_callback: Box::new(relocation),
    }
  }
}

impl<P> RePackablePacker for GrowableSelfContainedPacker<P>
where
  P: RePackablePacker + TexturePackerInit,
{
  type Input = P::Input;

  type PackOutput = P::PackOutput;

  fn pack_with_id(
    &mut self,
    input: Self::Input,
  ) -> Result<PackResultWithId<Self::PackOutput>, PackError> {
    self
      .packer
      .pack_and_check_grow(input, &mut self.on_grow, &mut self.relocation_callback)
  }

  fn unpack(&mut self, id: PackId) -> Result<(), UnpackError> {
    self.packer.unpack(id).map(|_| {})
  }
}
