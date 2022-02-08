use crate::*;
use rendiation_algebra::*;

impl ShaderGraphNodeType for AnyType {
  fn to_type() -> ShaderValueType {
    ShaderValueType::Never
  }
}

impl PrimitiveShaderGraphNodeType for bool {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Bool
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Bool(*self)
  }
}

impl PrimitiveShaderGraphNodeType for u32 {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Uint32
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Uint32(*self)
  }
}

impl PrimitiveShaderGraphNodeType for f32 {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Float32
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Float32(*self)
  }
}
impl ShaderGraphAttributeNodeType for f32 {}

impl PrimitiveShaderGraphNodeType for Vec2<f32> {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Vec2Float32
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Vec2Float32(*self)
  }
}
impl ShaderGraphAttributeNodeType for Vec2<f32> {}

impl PrimitiveShaderGraphNodeType for Vec3<f32> {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Vec3Float32
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Vec3Float32(*self)
  }
}
impl ShaderGraphAttributeNodeType for Vec3<f32> {}

impl PrimitiveShaderGraphNodeType for Vec4<f32> {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Vec4Float32
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Vec4Float32(*self)
  }
}
impl ShaderGraphAttributeNodeType for Vec4<f32> {}

impl ShaderGraphNodeType for ShaderSampler {
  fn to_type() -> ShaderValueType {
    ShaderValueType::Sampler
  }
}

impl PrimitiveShaderGraphNodeType for Mat4<f32> {
  fn to_primitive_type() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Mat4Float32
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Mat4Float32(*self)
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
  fn to_type() -> ShaderValueType {
    ShaderValueType::Texture
  }
}

impl Node<ShaderTexture> {
  pub fn sample(&self, sampler: Node<ShaderSampler>, position: Node<Vec2<f32>>) -> Node<Vec4<f32>> {
    ShaderGraphNodeExpr::TextureSampling(TextureSamplingNode {
      texture: self.handle(),
      sampler: sampler.handle(),
      position: position.handle(),
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
