use crate::{Handle, ResourceManager, ResourceWrap, SceneGraphBackend};

pub struct SceneShadingParameterGroupData<T: SceneGraphBackend> {
  pub gpu: T::ShadingParameterGroup,
  pub items: Vec<ShadingParameterType<T>>,
}

pub type ParameterHandle<T: SceneGraphBackend> = Handle<ResourceWrap<SceneShadingParameterGroupData<T>>>;
pub type UniformHandle<T: SceneGraphBackend> = Handle<ResourceWrap<T::UniformBuffer>>;
// pub type SamplerHandle<T: SceneGraphBackend> = Handle<ResourceWrap<T::Sampler>>;
// pub type TextureHandle<T: SceneGraphBackend> = Handle<ResourceWrap<T::Texture>>;

pub enum ShadingParameterType<T: SceneGraphBackend> {
  UniformBuffer(UniformHandle<T>),
  // Texture(Handle),
  // Sampler(Handle),
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
    index: ParameterHandle<T>,
  ) -> &mut ResourceWrap<SceneShadingParameterGroupData<T>> {
    self.shading_parameter_groups.get_mut(index).unwrap()
  }

  pub fn get_shading_param_group(
    &self,
    index: ParameterHandle<T>,
  ) -> &ResourceWrap<SceneShadingParameterGroupData<T>> {
    self.shading_parameter_groups.get(index).unwrap()
  }

  pub fn delete_shading_param_group(&mut self, index: ParameterHandle<T>) {
    self.shading_parameter_groups.remove(index);
  }
}

/// uniforms
impl<T: SceneGraphBackend> ResourceManager<T> {
  pub fn add_uniform(&mut self, gpu: T::UniformBuffer) -> &mut ResourceWrap<T::UniformBuffer> {
    ResourceWrap::new_wrap(&mut self.uniforms, gpu)
  }

  pub fn get_uniform_mut(
    &mut self,
    index: UniformHandle<T>,
  ) -> &mut ResourceWrap<T::UniformBuffer> {
    self.uniforms.get_mut(index).unwrap()
  }

  pub fn get_uniform(&self, index: UniformHandle<T>) -> &ResourceWrap<T::UniformBuffer> {
    self.uniforms.get(index).unwrap()
  }

  pub fn delete_uniform(&mut self, index: UniformHandle<T>) {
    self.uniforms.remove(index);
  }
}
