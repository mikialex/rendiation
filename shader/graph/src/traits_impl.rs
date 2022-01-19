use crate::{
  modify_graph, AnyType, Node, ShaderGraphAttributeNodeType, ShaderGraphConstableNodeType,
  ShaderGraphNodeData, ShaderGraphNodeType, ShaderSampler, ShaderTexture, TextureSamplingNode,
};
use rendiation_algebra::*;

impl ShaderGraphNodeType for AnyType {
  fn to_glsl_type() -> &'static str {
    unreachable!("Node can't created with type AnyType")
  }
}

impl ShaderGraphNodeType for f32 {
  fn to_glsl_type() -> &'static str {
    "float"
  }
}
impl ShaderGraphAttributeNodeType for f32 {}
impl ShaderGraphConstableNodeType for f32 {
  fn const_to_glsl(&self) -> String {
    let mut result = format!("{}", self);
    if result.contains('.') {
      result
    } else {
      result.push_str(".0");
      result
    }
  }
}

impl ShaderGraphNodeType for Vec2<f32> {
  fn to_glsl_type() -> &'static str {
    "vec2"
  }
}
impl ShaderGraphAttributeNodeType for Vec2<f32> {}
impl ShaderGraphConstableNodeType for Vec2<f32> {
  fn const_to_glsl(&self) -> String {
    format!(
      "vec2({}, {})",
      self.x.const_to_glsl(),
      self.y.const_to_glsl()
    )
  }
}

impl ShaderGraphNodeType for Vec3<f32> {
  fn to_glsl_type() -> &'static str {
    "vec3"
  }
}
impl ShaderGraphAttributeNodeType for Vec3<f32> {}
impl ShaderGraphConstableNodeType for Vec3<f32> {
  fn const_to_glsl(&self) -> String {
    format!(
      "vec3({}, {}, {})",
      self.x.const_to_glsl(),
      self.y.const_to_glsl(),
      self.z.const_to_glsl()
    )
  }
}

impl ShaderGraphNodeType for Vec4<f32> {
  fn to_glsl_type() -> &'static str {
    "vec4"
  }
}
impl ShaderGraphAttributeNodeType for Vec4<f32> {}
impl ShaderGraphConstableNodeType for Vec4<f32> {
  fn const_to_glsl(&self) -> String {
    format!(
      "vec4({}, {}, {}, {})",
      self.x.const_to_glsl(),
      self.y.const_to_glsl(),
      self.z.const_to_glsl(),
      self.w.const_to_glsl()
    )
  }
}

impl ShaderGraphNodeType for Mat4<f32> {
  fn to_glsl_type() -> &'static str {
    "mat4"
  }
}

impl ShaderGraphNodeType for Mat3<f32> {
  fn to_glsl_type() -> &'static str {
    "mat3"
  }
}

impl ShaderGraphNodeType for ShaderSampler {
  fn to_glsl_type() -> &'static str {
    "sampler"
  }
}

// impl ShaderGraphBindGroupItemProvider for ShaderSampler {
//   type ShaderGraphBindGroupItemInstance = Node<ShaderSampler>;

//   fn create_instance(
//     name: &'static str,
//     bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'_>,
//     stage: ShaderStage,
//   ) -> Self::ShaderGraphBindGroupItemInstance {
//     let node = bindgroup_builder.create_uniform_node::<ShaderSampler>(name);
//     bindgroup_builder.add_none_ubo(unsafe { node.handle.cast_type().into() }, stage);
//     node
//   }
// }

impl ShaderGraphNodeType for ShaderTexture {
  fn to_glsl_type() -> &'static str {
    "texture2D"
  }
}

impl Node<ShaderTexture> {
  pub fn sample(&self, sampler: Node<ShaderSampler>, position: Node<Vec2<f32>>) -> Node<Vec4<f32>> {
    ShaderGraphNodeData::TextureSampling(TextureSamplingNode {
      texture: *self,
      sampler: sampler,
      position: position,
    })
    .insert_graph()
  }
}

// impl ShaderGraphBindGroupItemProvider for ShaderTexture {
//   type ShaderGraphBindGroupItemInstance = Node<ShaderTexture>;

//   fn create_instance(
//     name: &'static str,
//     bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'_>,
//     stage: ShaderStage,
//   ) -> Self::ShaderGraphBindGroupItemInstance {
//     let node = bindgroup_builder.create_uniform_node::<ShaderTexture>(name);
//     bindgroup_builder.add_none_ubo(unsafe { node.handle.cast_type().into() }, stage);
//     node
//   }
// }
