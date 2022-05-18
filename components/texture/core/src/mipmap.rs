pub struct MipMap<T> {
  levels: Vec<T>,
}

impl<T: Texture2D> MipMap<T> {
  pub fn level_count(&self) -> usize {
    self.levels.len()
  }

  pub fn main_layer(&self) -> &T {
    &self.levels[0]
  }

  pub fn main_layer_mut(&self) -> &mut T {
    &mut self.levels[0]
  }

  /// validate the mip arrays' each layer size is a valid mipmap array
  pub fn validate_size(&self) -> bool {
    let mut previous_level = None;

    for level in self.levels {
      if let Some(previous) = previous_level {
        if previous.width / 2 != level.width || previous.height / 2 != level.height {
          return false;
        }
      };
      previous_level = level.into.size().into()
    }
    true
  }
}
