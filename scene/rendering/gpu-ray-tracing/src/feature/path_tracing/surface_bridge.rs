use std::hash::Hash;

use rendiation_lighting_transport::*;

use crate::*;

pub trait DevicePathTracingSurface: ShaderHashProvider + DynClone {
  fn build(&self, cx: &mut ShaderBindGroupBuilder) -> Box<dyn DevicePathTracingSurfaceInvocation>;
  fn bind(&self, cx: &mut BindingBuilder);
}
dyn_clone::clone_trait_object!(DevicePathTracingSurface);

pub trait DevicePathTracingSurfaceInvocation: DynClone {
  fn construct_shading_point(
    &self,
    sm_id: Node<u32>,
    normal: Node<Vec3<f32>>,
    uv: Node<Vec2<f32>>,
  ) -> Box<dyn DevicePathTracingSurfacePointInvocation>;
}
dyn_clone::clone_trait_object!(DevicePathTracingSurfaceInvocation);

pub trait DevicePathTracingSurfacePointInvocation {
  fn importance_sampling_brdf(
    &self,
    view_dir: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> RTSurfaceInteraction;
  fn eval_brdf(&self, view_dir: Node<Vec3<f32>>, light_dir: Node<Vec3<f32>>) -> Node<Vec3<f32>>;
}

pub struct RTSurfaceInteraction {
  pub sampling_dir: Node<Vec3<f32>>,
  pub brdf: Node<Vec3<f32>>,
  pub pdf: Node<f32>,
  pub surface_radiance: Node<Vec3<f32>>,
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
pub struct TestingMirrorSurfaceInvocationPoint {
  normal: Node<Vec3<f32>>,
}

impl DevicePathTracingSurfaceInvocation for TestingMirrorSurfaceInvocation {
  fn construct_shading_point(
    &self,
    _sm_id: Node<u32>,
    normal: Node<Vec3<f32>>,
    _uv: Node<Vec2<f32>>,
  ) -> Box<dyn DevicePathTracingSurfacePointInvocation> {
    Box::new(TestingMirrorSurfaceInvocationPoint { normal })
  }
}

impl DevicePathTracingSurfacePointInvocation for TestingMirrorSurfaceInvocationPoint {
  fn importance_sampling_brdf(
    &self,
    view_dir: Node<Vec3<f32>>,
    _sampler: &dyn DeviceSampler,
  ) -> RTSurfaceInteraction {
    RTSurfaceInteraction {
      sampling_dir: self.normal.reflect(-view_dir),
      brdf: val(Vec3::splat(0.5)),
      pdf: val(1.),
      surface_radiance: val(Vec3::zero()),
    }
  }

  // dirac dist
  fn eval_brdf(&self, _view_dir: Node<Vec3<f32>>, _light_dir: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
    unreachable!()
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
    self.material_accessor.len().hash(hasher); // todo, hash internal
  }
}

impl DevicePathTracingSurface for SceneSurfaceSupport {
  fn build(&self, cx: &mut ShaderBindGroupBuilder) -> Box<dyn DevicePathTracingSurfaceInvocation> {
    let mut reg = SemanticRegistry::default();
    self.textures.register_system_self_for_compute(cx, &mut reg);
    Box::new(SceneSurfaceSupportInvocation {
      reg: Arc::new(RwLock::new(reg)),
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
  reg: Arc<RwLock<SemanticRegistry>>,
  textures: GPUTextureBindingSystem,
  sm_to_material_type: ShaderReadonlyPtrOf<[u32]>,
  sm_to_material_id: ShaderReadonlyPtrOf<[u32]>,
  material_accessor: Arc<Vec<Box<dyn SceneMaterialSurfaceSupportInvocation>>>,
}

struct SceneSurfaceSupportInvocationPoint {
  normal: Node<Vec3<f32>>,
  surface: ShaderRtxPhysicalMaterial<
    ShaderLambertian,
    ShaderGGX,
    ShaderSmithGGXCorrelatedGeometryShadow,
    ShaderSchlick,
  >,
  emissive: Node<Vec3<f32>>,
}

impl DevicePathTracingSurfacePointInvocation for SceneSurfaceSupportInvocationPoint {
  fn importance_sampling_brdf(
    &self,
    view_dir: Node<Vec3<f32>>,
    sampler: &dyn DeviceSampler,
  ) -> RTSurfaceInteraction {
    let ShaderBRDFImportanceSampled {
      sample: light_dir,
      pdf,
      importance: brdf,
    } = self
      .surface
      .sample_light_dir_use_bsdf_importance(view_dir, self.normal, sampler);

    RTSurfaceInteraction {
      sampling_dir: light_dir,
      brdf,
      pdf,
      surface_radiance: self.emissive,
    }
  }

  fn eval_brdf(&self, view_dir: Node<Vec3<f32>>, light_dir: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
    self.surface.bsdf(view_dir, light_dir, self.normal)
  }
}

impl DevicePathTracingSurfaceInvocation for SceneSurfaceSupportInvocation {
  fn construct_shading_point(
    &self,
    sm_id: Node<u32>,
    normal: Node<Vec3<f32>>,
    uv: Node<Vec2<f32>>,
  ) -> Box<dyn DevicePathTracingSurfacePointInvocation> {
    let material_ty = self.sm_to_material_type.index(sm_id).load();
    let material_id = self.sm_to_material_id.index(sm_id).load();

    let physical_desc = zeroed_val::<ShaderPhysicalShading>().make_local_var();

    // find material impl by id, and construct surface
    let reg = self.reg.read();
    let mut switch = switch_by(material_ty);
    for (i, m) in self.material_accessor.iter().enumerate() {
      switch = switch.case(i as u32, || {
        // we deep clone this to avoid variable wrongly reused between different case scope.
        let mut reg = (*reg).clone();
        m.inject_material_info(&mut reg, material_id, uv, &self.textures);
        let s = PhysicalShading::construct_shading_impl(&reg);
        physical_desc.store(s.construct());
      });
    }

    switch.end_with_default(|| {
      let registry = SemanticRegistry::default();
      // just create from an empty registry to get default value.
      let s = PhysicalShading::construct_shading_impl(&registry);
      physical_desc.store(s.construct());
    });

    let physical_desc = physical_desc.load().expand();

    let roughness = physical_desc.linear_roughness;
    let specular = ShaderSpecular {
      f0: physical_desc.f0,
      normal_distribution_model: ShaderGGX { roughness },
      geometric_shadow_model: ShaderSmithGGXCorrelatedGeometryShadow { roughness },
      fresnel_model: ShaderSchlick,
    };

    let surface = ShaderRtxPhysicalMaterial {
      diffuse: ShaderLambertian {
        albedo: physical_desc.albedo,
      },
      specular,
    };

    Box::new(SceneSurfaceSupportInvocationPoint {
      normal,
      surface,
      emissive: physical_desc.emissive,
    })
  }
}
