use std::hash::Hash;

use crate::*;

#[derive(Default, Clone)]
pub struct RayTracingTextureIO {
  targets: Arc<RwLock<FastHashMap<TypeId, StorageTextureReadWrite<GPU2DTextureView>>>>,
}

impl ShaderHashProvider for RayTracingTextureIO {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.targets.read().iter().for_each(|(k, _)| {
      k.hash(hasher);
    });
  }
}

impl RayTracingCustomCtxProvider for RayTracingTextureIO {
  type Invocation = FrameOutputInvocation;

  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation {
    let targets = self
      .targets
      .read()
      .iter()
      .map(|(k, v)| (*k, cx.bind_by(v)))
      .collect();

    FrameOutputInvocation { targets }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    // we assume the iter order of hashmap will not changed between build_invocation, hash_pipeline and here.
    self.targets.read().iter().for_each(|(_, v)| {
      builder.bind(v);
    });
  }
}

pub trait RayTracingOutputTargetSemantic: 'static {}

impl RayTracingTextureIO {
  /// before each time rendering is triggered, any texture io resource should be installed into system.
  /// and the target should be taken out by [[take_output_target]] as soon as rendering is done.
  ///
  /// todo, support different output target type
  pub fn install_output_target<T: RayTracingOutputTargetSemantic>(&self, target: GPU2DTextureView) {
    self.targets.write().insert(
      TypeId::of::<T>(),
      target.into_storage_texture_view_readwrite().unwrap(),
    );
  }

  pub fn take_output_target<T: RayTracingOutputTargetSemantic>(&self) -> GPU2DTextureView {
    self
      .targets
      .write()
      .remove(&TypeId::of::<T>())
      .unwrap()
      .texture
  }
}

#[derive(Clone)]
pub struct FrameOutputInvocation {
  // todo, separate read and write capabilities
  targets: FastHashMap<TypeId, HandleNode<ShaderStorageTextureRW2D>>,
}

impl FrameOutputInvocation {
  pub fn read_output<T: RayTracingOutputTargetSemantic>(
    &self,
    position: Node<Vec2<u32>>,
  ) -> Node<Vec4<f32>> {
    self
      .targets
      .get(&TypeId::of::<T>())
      .unwrap()
      .load_texel(position, val(0))
  }

  pub fn write_output<T: RayTracingOutputTargetSemantic>(
    &self,
    position: Node<Vec2<u32>>,
    value: Node<Vec4<f32>>,
  ) {
    self
      .targets
      .get(&TypeId::of::<T>())
      .unwrap()
      .write_texel(position, value)
  }
}
