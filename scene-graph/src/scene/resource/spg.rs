use crate::{Index, ResourceManager, SceneGraphBackEnd};

pub struct SceneShadingParameterGroup<T: SceneGraphBackEnd> {
  index: Index,
  items: Vec<(Index, ShadingParameterType)>,
  pub gpu: T::ShadingParameterGroup, // todo private
}

pub enum ShadingParameterType {
  UniformBuffer,
  Texture,
  Sampler,
}

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn create_shading_param_group(
    &mut self,
    items: Vec<(Index, ShadingParameterType)>,
  ) -> &mut SceneShadingParameterGroup<T> {
    todo!()
    // let wrapped = SceneGeometry {
    //   index: Index::from_raw_parts(0, 0),
    //   data: Box::new(shading_param_group),
    // };
    // let index = self.geometries.insert(wrapped);
    // let g = self.get_shading_param_group_mut(index);
    // g.index = index;
    // g
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
