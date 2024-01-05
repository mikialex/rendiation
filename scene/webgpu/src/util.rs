use crate::*;

pub struct ValueIDGenerator<T> {
  inner: FastHashMap<T, usize>,
  values: Vec<T>,
}

impl<T> Default for ValueIDGenerator<T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
      values: Default::default(),
    }
  }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct ValueID<T> {
  value: usize,
  ty: PhantomData<T>,
}

impl<T> Clone for ValueID<T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<T> Copy for ValueID<T> {}

impl<T> ValueIDGenerator<T>
where
  T: Eq + Hash + Clone,
{
  pub fn get_uuid(&mut self, v: &T) -> ValueID<T> {
    let count = self.values.len();
    let id = self.inner.raw_entry_mut().from_key(v).or_insert_with(|| {
      self.values.push(v.clone());
      (v.clone(), count)
    });
    ValueID {
      value: *id.1,
      ty: PhantomData,
    }
  }

  pub fn get_value(&self, id: ValueID<T>) -> Option<&T> {
    self.values.get(id.value)
  }
}

fn convert_wrap(mode: rendiation_texture::AddressMode) -> AddressMode {
  match mode {
    rendiation_texture::AddressMode::ClampToEdge => AddressMode::ClampToEdge,
    rendiation_texture::AddressMode::Repeat => AddressMode::Repeat,
    rendiation_texture::AddressMode::MirrorRepeat => AddressMode::MirrorRepeat,
  }
}

fn convert_filter(mode: rendiation_texture::FilterMode) -> FilterMode {
  match mode {
    rendiation_texture::FilterMode::Nearest => FilterMode::Nearest,
    rendiation_texture::FilterMode::Linear => FilterMode::Linear,
  }
}

pub trait SamplerConvertExt<'a> {
  fn into_gpu(self) -> SamplerDescriptor<'a>;
}

impl<'a> SamplerConvertExt<'a> for rendiation_texture::TextureSampler {
  fn into_gpu(self) -> SamplerDescriptor<'a> {
    SamplerDescriptor {
      label: None,
      address_mode_u: convert_wrap(self.address_mode_u),
      address_mode_v: convert_wrap(self.address_mode_v),
      address_mode_w: convert_wrap(self.address_mode_w),
      mag_filter: convert_filter(self.mag_filter),
      min_filter: convert_filter(self.min_filter),
      mipmap_filter: convert_filter(self.mipmap_filter),
      ..Default::default()
    }
  }
}
