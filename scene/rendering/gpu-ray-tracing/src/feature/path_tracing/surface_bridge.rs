use std::hash::Hash;

use rendiation_lighting_transport::{PhysicalShading, ShaderPhysicalShading};

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

#[derive(Clone)]
pub struct SceneSurfaceSupport {
  pub textures: GPUTextureBindingSystem,
  pub sm_to_material_type: StorageBufferReadonlyDataView<[u32]>,
  pub sm_to_material_id: StorageBufferReadonlyDataView<[u32]>,
  pub material_accessor: Arc<Vec<Box<dyn SceneMaterialSurfaceSupport>>>,
}

impl ShaderHashProvider for SceneSurfaceSupport {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.textures.hash_pipeline(hasher);
    self.material_accessor.len().hash(hasher);
  }
}

impl DevicePathTracingSurface for SceneSurfaceSupport {
  fn build(&self, cx: &mut ShaderBindGroupBuilder) -> Box<dyn DevicePathTracingSurfaceInvocation> {
    self.textures.register_system_self_for_compute(cx);
    Box::new(SceneSurfaceSupportInvocation {
      textures: self.textures.clone(),
      sm_to_material_type: cx.bind_by(&self.sm_to_material_type),
      sm_to_material_id: cx.bind_by(&self.sm_to_material_id),
      material_accessor: Arc::new(self.material_accessor.iter().map(|m| m.build(cx)).collect()),
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    self.textures.bind_system_self(cx);
    cx.bind(&self.sm_to_material_type);
    cx.bind(&self.sm_to_material_id);
    for m in self.material_accessor.iter() {
      m.bind(cx);
    }
  }
}

#[derive(Clone)]
struct SceneSurfaceSupportInvocation {
  textures: GPUTextureBindingSystem,
  sm_to_material_type: ShaderReadonlyPtrOf<[u32]>,
  sm_to_material_id: ShaderReadonlyPtrOf<[u32]>,
  material_accessor: Arc<Vec<Box<dyn SceneMaterialSurfaceSupportInvocation>>>,
}

impl DevicePathTracingSurfaceInvocation for SceneSurfaceSupportInvocation {
  fn importance_sampling_brdf(
    &self,
    sm_id: Node<u32>,
    incident_dir: Node<Vec3<f32>>,
    normal: Node<Vec3<f32>>,
    uv: Node<Vec2<f32>>,
  ) -> RTSurfaceInteraction {
    let material_ty = self.sm_to_material_type.index(sm_id).load();
    let material_id = self.sm_to_material_id.index(sm_id).load();

    let surface = zeroed_val::<ShaderPhysicalShading>().make_local_var();

    // find material impl by id, and construct surface
    let mut switch = switch_by(material_ty);
    for (i, m) in self.material_accessor.iter().enumerate() {
      switch = switch.case(i as u32, || {
        let mut registry = SemanticRegistry::default();
        m.inject_material_info(&mut registry, material_id, uv, &self.textures);
        let s = PhysicalShading::construct_shading_impl(&registry);
        surface.store(s.construct());
      });
    }

    switch.end_with_default(|| {
      let registry = SemanticRegistry::default();
      // just create from an empty registry to get default value.
      let s = PhysicalShading::construct_shading_impl(&registry);
      surface.store(s.construct());
    });

    // todo, surface sample and compute brdf
    let _surface = surface.load();

    RTSurfaceInteraction {
      sampling_dir: normal.reflect(incident_dir),
      brdf: val(Vec3::splat(0.5)),
      pdf: val(1.),
    }
  }
}
