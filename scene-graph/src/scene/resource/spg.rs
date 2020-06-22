use crate::{Handle, ResourceManager, ResourceWrap, SceneGraphBackend};

pub struct SceneShadingParameterGroupData<T: SceneGraphBackend> {
  pub gpu: T::ShadingParameterGroup,
  pub items: Vec<ShadingParameterType>,
}

pub type ParameterHandle<T: SceneGraphBackend> = Handle<ResourceWrap<T::ShadingParameterGroup>>;

pub enum ShadingParameterType {
  UniformBuffer(Handle),
  Texture(Handle),
  Sampler(Handle),
  SampledTexture(Handle),
}

impl<T: SceneGraphBackend> ResourceManager<T> {
  pub fn add_shading_param_group(
    &mut self,
    resource: SceneShadingParameterGroupData<T>,
  ) -> &mut ResourceWrap<SceneShadingParameterGroupData<T>> {
    ResourceWrap::new_wrap(&mut self.shading_parameter_groups, resource)
  }

  pub fn get_shading_param_group_mut(
    &mut self,
    index: Handle,
  ) -> &mut ResourceWrap<SceneShadingParameterGroupData<T>> {
    self.shading_parameter_groups.get_mut(index).unwrap()
  }

  pub fn get_shading_param_group(
    &self,
    index: Handle,
  ) -> &ResourceWrap<SceneShadingParameterGroupData<T>> {
    self.shading_parameter_groups.get(index).unwrap()
  }

  pub fn delete_shading_param_group(&mut self, index: Handle) {
    self.shading_parameter_groups.remove(index);
  }
}

/// uniforms
impl<T: SceneGraphBackend> ResourceManager<T> {
  pub fn add_uniform(&mut self, gpu: T::UniformBuffer) -> &mut ResourceWrap<T::UniformBuffer> {
    ResourceWrap::new_wrap(&mut self.uniforms, gpu)
  }

  pub fn get_uniform_mut(&mut self, index: Handle) -> &mut ResourceWrap<T::UniformBuffer> {
    self.uniforms.get_mut(index).unwrap()
  }

  pub fn get_uniform(&self, index: Handle) -> &ResourceWrap<T::UniformBuffer> {
    self.uniforms.get(index).unwrap()
  }

  pub fn delete_uniform(&mut self, index: Handle) {
    self.uniforms.remove(index);
  }
}
