

struct SceneShadingDescriptor{
  pub vertex_shader_str: String,
  pub frag_shader_str: String,
  // .. blend state stuff
}


/// webgpu => pipeline
/// webgl => program
struct SceneShading{
  index: Index,
  parameters: Vec<Option<Index>>,
  handle_index: Index,
}

/// webgpu => bindgroup
/// webgl => nothing
struct SceneShadingParameterGroup{
  index: Index,
  items: Vec<(Index, ShadingParameterType)>,
  handle_index: Index,
}

enum ShadingParameterType{
  UniformBuffer,
  Texture,
  Sampler,
}

/// webgpu => buffer
/// webgl => uniform / ubo
struct UniformBuffer{

}

impl SceneGraph{
  pub fn create_shading(&mut self, shading: &SceneShadingDescriptor) -> &mut SceneShading{

  }

  pub fn create_shading_parameter_group() -> &mut SceneShadingParameterGroup{

  }


}