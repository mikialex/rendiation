use crate::*;

/// for simplicity we not expect shader variant, so skip shader hashing
pub trait SceneMaterialSurfaceSupport {
  fn build(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn SceneMaterialSurfaceSupportInvocation>;
  fn bind(&self, cx: &mut BindingBuilder);
}

pub trait SceneMaterialSurfaceSupportInvocation {
  fn inject_material_info(
    &self,
    reg: &mut SemanticRegistry,
    uv: Node<Vec2<f32>>,
    textures: &GPUTextureBindingSystem,
  );
}
