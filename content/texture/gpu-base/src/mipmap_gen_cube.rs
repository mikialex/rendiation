use crate::*;

pub trait MipmapCubeReducer: Send + Sync {
  fn reduce(
    &self,
    source: HandleNode<ShaderTextureCube>,
    sampler: HandleNode<ShaderSampler>,
    current_uv: Node<Vec2<f32>>,
    current_face_index: u8,
    current_world_direction: Node<Vec3<f32>>,
    texel_size: Node<Vec2<f32>>,
  );
}
