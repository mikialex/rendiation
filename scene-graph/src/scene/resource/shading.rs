use crate::{Handle, RALBackend, ResourceManager, ResourceWrap};

// pub struct SceneShadingData<T: RALBackend> {
//   gpu: T::Shading,
//   parameters: Vec<ParameterHandle<T>>,
// }

// impl<T: RALBackend> SceneShadingData<T> {
//   pub fn new(gpu: T::Shading) -> Self {
//     Self {
//       gpu,
//       parameters: Vec::new(),
//     }
//   }

//   pub fn gpu(&self) -> &T::Shading {
//     &self.gpu
//   }

//   pub fn push_parameter(mut self, index: BindgroupPair<T>) -> Self {
//     self.parameters.push(index);
//     self
//   }

//   pub fn get_parameters_count(&self) -> usize {
//     self.parameters.len()
//   }

//   pub fn get_parameter(&self, index: usize) -> BindgroupPair<T> {
//     self.parameters[index]
//   }
// }

pub type ShadingHandle<T> = Handle<ResourceWrap<SceneShadingData<T>>>;

impl<T: RALBackend> ResourceManager<T> {
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
