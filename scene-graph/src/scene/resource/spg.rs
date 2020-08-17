use crate::{Handle, RALBackend, ResourceManager, ResourceWrap};
use rendiation_ral::*;

pub struct SceneShadingParameterGroupData<T: RALBackend> {
  type_id: ParameterGroupTypeId,
  gpu: T::ShadingParameterGroup,
  items: Vec<(UniformTypeId, ShadingParameterType<T>)>,
}

impl<T: RALBackend> SceneShadingParameterGroupData<T> {
  pub fn new(type_id: ParameterGroupTypeId, gpu: T::ShadingParameterGroup) -> Self {
    Self {
      type_id,
      gpu,
      items: Vec::new(),
    }
  }

  pub fn type_id(&self) -> ParameterGroupTypeId {
    self.type_id
  }

  pub fn gpu(&self) -> &T::ShadingParameterGroup {
    &self.gpu
  }

  pub fn items(&self) -> &Vec<(UniformTypeId, ShadingParameterType<T>)> {
    &self.items
  }
}

pub type ParameterHandle<T> = Handle<ResourceWrap<SceneShadingParameterGroupData<T>>>;
pub type UniformHandle<T> = Handle<ResourceWrap<<T as RALBackend>::UniformBuffer>>;
pub type UniformValueHandle<T> = Handle<ResourceWrap<<T as RALBackend>::UniformValue>>;
pub type SamplerHandle<T> = Handle<ResourceWrap<<T as RALBackend>::Sampler>>;
pub type TextureHandle<T> = Handle<ResourceWrap<<T as RALBackend>::Texture>>;
pub type SampledTextureHandle<T> = Handle<ResourceWrap<<T as RALBackend>::Texture>>;

pub enum ShadingParameterType<T: RALBackend> {
  UniformValue(UniformValueHandle<T>),
  UniformBuffer(UniformHandle<T>),
  Texture(SamplerHandle<T>),
  Sampler(TextureHandle<T>),
  SampledTexture(SampledTextureHandle<T>),
}

impl<T: RALBackend> ResourceManager<T> {
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
impl<T: RALBackend> ResourceManager<T> {
  pub fn add_uniform(&mut self, gpu: T::UniformBuffer) -> &mut ResourceWrap<T::UniformBuffer> {
    ResourceWrap::new_wrap(&mut self.uniform_buffers, gpu)
  }

  pub fn get_uniform_mut(
    &mut self,
    index: UniformHandle<T>,
  ) -> &mut ResourceWrap<T::UniformBuffer> {
    self.uniform_buffers.get_mut(index).unwrap()
  }

  pub fn get_uniform(&self, index: UniformHandle<T>) -> &ResourceWrap<T::UniformBuffer> {
    self.uniform_buffers.get(index).unwrap()
  }

  pub fn delete_uniform(&mut self, index: UniformHandle<T>) {
    self.uniform_buffers.remove(index);
  }
}

/// uniform values
impl<T: RALBackend> ResourceManager<T> {
  pub fn add_uniform_value(&mut self, gpu: T::UniformValue) -> &mut ResourceWrap<T::UniformValue> {
    ResourceWrap::new_wrap(&mut self.uniform_values, gpu)
  }

  pub fn get_uniform_value_mut(
    &mut self,
    index: UniformValueHandle<T>,
  ) -> &mut ResourceWrap<T::UniformValue> {
    self.uniform_values.get_mut(index).unwrap()
  }

  pub fn get_uniform_value(&self, index: UniformValueHandle<T>) -> &ResourceWrap<T::UniformValue> {
    self.uniform_values.get(index).unwrap()
  }

  pub fn delete_uniform_value(&mut self, index: UniformValueHandle<T>) {
    self.uniform_values.remove(index);
  }
}
