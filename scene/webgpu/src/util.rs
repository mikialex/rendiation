use crate::*;

pub struct ValueIDGenerator<T> {
  inner: HashMap<T, usize>,
  values: Vec<T>,
}

impl<T> Default for ValueIDGenerator<T> {
  fn default() -> Self {
    Self {
      inner: HashMap::default(),
      values: Vec::new(),
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
    Self {
      value: self.value,
      ty: self.ty,
    }
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

pub struct OptionalRenderComponent<T>(Option<T>);

impl<T> From<Option<T>> for OptionalRenderComponent<T> {
  fn from(value: Option<T>) -> Self {
    Self(value)
  }
}

impl<T: ShaderHashProvider> ShaderHashProvider for OptionalRenderComponent<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    if let Some(v) = &self.0 {
      v.hash_pipeline(hasher)
    }
  }
}

impl<T: ShaderGraphProvider> ShaderGraphProvider for OptionalRenderComponent<T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    if let Some(v) = &self.0 {
      v.build(builder)?
    }
    Ok(())
  }

  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    if let Some(v) = &self.0 {
      v.post_build(builder)?
    }
    Ok(())
  }
}
