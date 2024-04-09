use fast_hash_collection::FastHashMap;

use crate::*;

pub struct GrowablePacker<P: RePackablePacker + TexturePackerInit> {
  packer: P,
  current_config: P::Config,
  result: FastHashMap<PackId, (P::Input, P::PackOutput)>,
  on_grow: Box<dyn FnMut(P::Config) -> Option<P::Config> + Send + Sync>,
  relocation_callback: Box<dyn FnMut(PackResultRelocation<P::PackOutput>) + Send + Sync>,
}

impl<P: RePackablePacker + TexturePackerInit> GrowablePacker<P> {
  pub fn new(
    init: P::Config,
    grow: impl FnMut(P::Config) -> Option<P::Config> + 'static + Send + Sync,
    relocation: impl FnMut(PackResultRelocation<P::PackOutput>) + 'static + Send + Sync,
  ) -> Self {
    Self {
      packer: P::init_by_config(init.clone()),
      current_config: init,
      result: Default::default(),
      on_grow: Box::new(grow),
      relocation_callback: Box::new(relocation),
    }
  }
}

pub struct PackResultRelocation<T> {
  pub previous: PackResultWithId<T>,
  pub new: PackResultWithId<T>,
}

impl<P> RePackablePacker for GrowablePacker<P>
where
  P: RePackablePacker + TexturePackerInit,
{
  type Input = P::Input;

  type PackOutput = P::PackOutput;

  fn pack_with_id(
    &mut self,
    input: Self::Input,
  ) -> Result<PackResultWithId<Self::PackOutput>, PackError> {
    loop {
      if let Ok(r) = self.packer.pack_with_id(input.clone()) {
        self.result.insert(r.id, (input.clone(), r.result));
      } else if let Some(new_config) = (self.on_grow)(self.current_config.clone()) {
        // todo, should we expose the current allocation info to avoid loop grow?
        self.packer = P::init_by_config(new_config);

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
          let new = PackResultWithId {
            result: new_result.result,
            id: new_result.id,
          };
          (self.relocation_callback)(PackResultRelocation { previous, new });
        }
      } else {
        return Err(PackError::SpaceNotEnough);
      }
    }
  }

  fn unpack(&mut self, id: PackId) -> Result<(), UnpackError> {
    self.packer.unpack(id)?;
    self.result.remove(&id);
    Ok(())
  }
}
