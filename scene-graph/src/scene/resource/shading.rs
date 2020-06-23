use crate::{Handle, ParameterHandle, ResourceManager, ResourceWrap, SceneGraphBackend};

pub struct SceneShadingData<T: SceneGraphBackend> {
  pub gpu: T::Shading,
  pub parameters: Vec<ParameterHandle<T>>,
}

impl<T: SceneGraphBackend> SceneShadingData<T> {
  pub fn push_parameter(&mut self, index: ParameterHandle<T>) {
    self.parameters.push(index);
  }

  pub fn get_parameters_count(&self) -> usize {
    self.parameters.len()
  }

  pub fn get_parameter(&self, index: usize) -> ParameterHandle<T> {
    self.parameters[index]
  }
}

pub type ShadingHandle<T: SceneGraphBackend> = Handle<ResourceWrap<SceneShadingData<T>>>;

impl<T: SceneGraphBackend> ResourceManager<T> {
  pub fn add_shading(
    &mut self,
    resource: SceneShadingData<T>,
  ) -> &mut ResourceWrap<SceneShadingData<T>> {
    ResourceWrap::new_wrap(&mut self.shadings, resource)
  }

  pub fn get_shading_mut(
    &mut self,
    index: ShadingHandle<T>,
  ) -> &mut ResourceWrap<SceneShadingData<T>> {
    self.shadings.get_mut(index).unwrap()
  }

  pub fn get_shading(&self, index: ShadingHandle<T>) -> &ResourceWrap<SceneShadingData<T>> {
    self.shadings.get(index).unwrap()
  }

  pub fn delete_shading(&mut self, index: ShadingHandle<T>) {
    self.shadings.remove(index);
  }
}
