pub trait ShaderInputNode {}

pub struct UniformNode {
  name: String,
}

impl ShaderInputNode for UniformNode {}

pub struct AttributeNode {
  name: String,
}

impl ShaderInputNode for AttributeNode {}

pub enum NodeType {
  Float,
  Vec2,
  Vec3,
  Vec4,
}
