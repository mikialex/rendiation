use crate::{Index, ResourceManager, SceneGraphBackend, ResourceWrap};

pub struct SceneShadingData<T: SceneGraphBackend>{
  pub gpu: T::Shading,
  pub parameters: Vec<Index>,
}

impl<T: SceneGraphBackend> SceneShadingData<T> {
  pub fn push_parameter(&mut self, index: Index) {
    self.parameters.push(index);
  }

  pub fn get_parameters_count(&self) -> usize {
    self.parameters.len()
  }

  pub fn get_parameter(&self, index: usize) -> Index {
    self.parameters[index]
  }
}

impl<T: SceneGraphBackend> ResourceManager<T> {
  pub fn add_shading(&mut self, resource: SceneShadingData<T>) -> &mut ResourceWrap<SceneShadingData<T>> {
    ResourceWrap::new_wrap(&mut self.shadings, resource)
  }


  pub fn get_shading_mut(&mut self, index: Index) -> &mut ResourceWrap<SceneShadingData<T>> {
    self.shadings.get_mut(index).unwrap()
  }

  pub fn get_shading(&self, index: Index) -> &ResourceWrap<SceneShadingData<T>> {
    self.shadings.get(index).unwrap()
  }

  pub fn delete_shading(&mut self, index: Index) {
    self.shadings.remove(index);
  }
}
