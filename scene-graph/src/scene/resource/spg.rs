use crate::{Index, ResourceManager, SceneGraphBackEnd, ResouceWrap};

pub struct SceneShadingParameterGroup<T: SceneGraphBackEnd> {
  index: Index,
  // items: Vec<(Index, ShadingParameterType)>, // todo
  gpu: T::ShadingParameterGroup, // todo private
}

impl<T: SceneGraphBackEnd> SceneShadingParameterGroup<T> {
  pub fn gpu(&self) -> &T::ShadingParameterGroup {
    &self.gpu
  }

  pub fn index(&self) -> Index {
    self.index
  }
}

pub enum ShadingParameterType {
  UniformBuffer,
  Texture,
  Sampler,
}

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn create_shading_param_group(
    &mut self,
    gpu: T::ShadingParameterGroup,
    // items: Vec<(Index, ShadingParameterType)>,
  ) -> &mut SceneShadingParameterGroup<T> {
    let wrapped = SceneShadingParameterGroup {
      index: Index::from_raw_parts(0, 0),
      // items: Vec::new(),
      gpu,
    };
    let index = self.shading_parameter_groups.insert(wrapped);
    let p = self.get_shading_param_group_mut(index);
    p.index = index;
    p
  }

  pub fn get_shading_param_group_mut(
    &mut self,
    index: Index,
  ) -> &mut SceneShadingParameterGroup<T> {
    self.shading_parameter_groups.get_mut(index).unwrap()
  }

  pub fn get_shading_param_group(&self, index: Index) -> &SceneShadingParameterGroup<T> {
    self.shading_parameter_groups.get(index).unwrap()
  }

  pub fn delete_shading_param_group(&mut self, index: Index) {
    self.shading_parameter_groups.remove(index);
  }
}

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
