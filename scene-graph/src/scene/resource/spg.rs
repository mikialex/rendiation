use crate::{Index, ResourceManager, SceneGraphBackEnd, ResourceWrap};

pub struct SceneShadingParameterGroupData<T: SceneGraphBackEnd>{
  pub gpu: T::ShadingParameterGroup,
  pub items: Vec<ShadingParameterType>,
}

pub enum ShadingParameterType {
  UniformBuffer(Index),
  Texture(Index),
  Sampler(Index),
  SampledTexture(Index),
}

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn add_shading_param_group(
    &mut self,
    resource: SceneShadingParameterGroupData<T>
  ) -> &mut ResourceWrap<SceneShadingParameterGroupData<T>> {
    ResourceWrap::new_wrap(&mut self.shading_parameter_groups, resource)
  }

  pub fn get_shading_param_group_mut(
    &mut self,
    index: Index,
  ) -> &mut ResourceWrap<SceneShadingParameterGroupData<T>> {
    self.shading_parameter_groups.get_mut(index).unwrap()
  }

  pub fn get_shading_param_group(&self, index: Index) -> &ResourceWrap<SceneShadingParameterGroupData<T>> {
    self.shading_parameter_groups.get(index).unwrap()
  }

  pub fn delete_shading_param_group(&mut self, index: Index) {
    self.shading_parameter_groups.remove(index);
  }
}

/// uniforms
impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn add_uniform(&mut self, gpu: T::UniformBuffer) -> &mut ResourceWrap<T::UniformBuffer> {
    ResourceWrap::new_wrap(&mut self.uniforms, gpu)
  }

  pub fn get_uniform_mut(&mut self, index: Index) -> &mut ResourceWrap<T::UniformBuffer> {
    self.uniforms.get_mut(index).unwrap()
  }

  pub fn get_uniform(&self, index: Index) -> &ResourceWrap<T::UniformBuffer> {
    self.uniforms.get(index).unwrap()
  }

  pub fn delete_uniform(&mut self, index: Index) {
    self.uniforms.remove(index);
  }
}
