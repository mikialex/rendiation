use crate::*;

pub trait DevicePathTracingSurface: ShaderHashProvider + DynClone {
  fn build(&self, cx: &mut ShaderBindGroupBuilder) -> Box<dyn DevicePathTracingSurfaceInvocation>;
  fn bind(&self, cx: &mut BindingBuilder);
}
dyn_clone::clone_trait_object!(DevicePathTracingSurface);

pub trait DevicePathTracingSurfaceInvocation: DynClone {
  fn importance_sampling_brdf(
    &self,
    scene_model_id: Node<u32>,
    incident_dir: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
    uv: Node<Vec2<f32>>,
  ) -> RTSurfaceInteraction;
}
dyn_clone::clone_trait_object!(DevicePathTracingSurfaceInvocation);

pub struct RTSurfaceInteraction {
  pub sampling_dir: Node<Vec3<f32>>,
  pub brdf: Node<Vec3<f32>>,
  pub pdf: Node<f32>,
}

#[derive(Clone)]
pub struct TestingMirrorSurface;
impl ShaderHashProvider for TestingMirrorSurface {
  shader_hash_type_id! {}
}
impl DevicePathTracingSurface for TestingMirrorSurface {
  fn build(&self, _: &mut ShaderBindGroupBuilder) -> Box<dyn DevicePathTracingSurfaceInvocation> {
    Box::new(TestingMirrorSurfaceInvocation)
  }
  fn bind(&self, _: &mut BindingBuilder) {}
}
#[derive(Clone)]
pub struct TestingMirrorSurfaceInvocation;

impl DevicePathTracingSurfaceInvocation for TestingMirrorSurfaceInvocation {
  fn importance_sampling_brdf(
    &self,
    _: Node<u32>,
    incident_dir: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
    _: Node<Vec2<f32>>,
  ) -> RTSurfaceInteraction {
    RTSurfaceInteraction {
      sampling_dir: normal.reflect(incident_dir),
      brdf: val(Vec3::splat(0.5)),
      pdf: val(1.),
    }
  }
}

// struct SceneSurfaceSupport {
//   textures: GPUTextureBindingSystem,
//   material_type: ShaderPtrOf<[u32]>,
//   material_accessor: Vec<u32>,
// }
