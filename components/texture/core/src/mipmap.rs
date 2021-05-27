pub struct MipMap<T> {
  maps: Vec<T>,
}

impl<T> MipMap<T> {
  pub fn levels(&self) -> usize {
    self.maps.len()
  }
}
