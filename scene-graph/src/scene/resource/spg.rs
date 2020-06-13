use crate::{Index, ResourceManager, SceneGraphBackEnd, ResouceWrap};

pub struct SceneShadingParameterGroupData<T: SceneGraphBackEnd>{
  pub gpu: T::ShadingParameterGroup,
  pub items: Vec<(Index, ShadingParameterType)>,
}

pub enum ShadingParameterType {
  UniformBuffer,
  Texture,
  Sampler,
}

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn add_shading_param_group(
    &mut self,
    resource: SceneShadingParameterGroupData<T>
  ) -> &mut ResouceWrap<SceneShadingParameterGroupData<T>> {
    ResouceWrap::new_wrap(&mut self.shading_parameter_groups, resource)
  }

  pub fn get_shading_param_group_mut(
    &mut self,
    index: Index,
  ) -> &mut ResouceWrap<SceneShadingParameterGroupData<T>> {
    self.shading_parameter_groups.get_mut(index).unwrap()
  }

  pub fn get_shading_param_group(&self, index: Index) -> &ResouceWrap<SceneShadingParameterGroupData<T>> {
    self.shading_parameter_groups.get(index).unwrap()
  }

  pub fn delete_shading_param_group(&mut self, index: Index) {
    self.shading_parameter_groups.remove(index);
  }
}

/// uniforms
impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn add_uniform(&mut self, gpu: T::UniformBuffer) -> &mut ResouceWrap<T::UniformBuffer> {
    ResouceWrap::new_wrap(&mut self.uniforms, gpu)
  }

  pub fn get_uniform_mut(&mut self, index: Index) -> &mut ResouceWrap<T::UniformBuffer> {
    self.uniforms.get_mut(index).unwrap()
  }

  pub fn get_uniform(&self, index: Index) -> &ResouceWrap<T::UniformBuffer> {
    self.uniforms.get(index).unwrap()
  }

  pub fn delete_uniform(&mut self, index: Index) {
    self.uniforms.remove(index);
  }
}
