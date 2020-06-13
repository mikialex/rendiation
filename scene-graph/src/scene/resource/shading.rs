use crate::{Index, ResourceManager, SceneGraphBackEnd, ResouceWrap};

pub struct SceneShadingData<T: SceneGraphBackEnd>{
  pub gpu: T::Shading,
  pub parameters: Vec<Index>,
}

impl<T: SceneGraphBackEnd> SceneShadingData<T> {
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

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn add_shading(&mut self, resource: SceneShadingData<T>) -> &mut ResouceWrap<SceneShadingData<T>> {
    ResouceWrap::new_wrap(&mut self.shadings, resource)
  }


  pub fn get_shading_mut(&mut self, index: Index) -> &mut ResouceWrap<SceneShadingData<T>> {
    self.shadings.get_mut(index).unwrap()
  }

  pub fn get_shading(&self, index: Index) -> &ResouceWrap<SceneShadingData<T>> {
    self.shadings.get(index).unwrap()
  }

  pub fn delete_shading(&mut self, index: Index) {
    self.shadings.remove(index);
  }
}
