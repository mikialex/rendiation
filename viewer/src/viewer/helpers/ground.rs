use __core::{any::Any, hash::Hash};
use rendiation_scene_core::{IdentityMapper, SceneItemRef};
use rendiation_scene_webgpu::{
  generate_quad, CameraGPU, DrawcallEmitter, PassContentWithCamera, RenderComponent,
  RenderComponentAny, RenderEmitter,
};
use shadergraph::*;
use webgpu::{
  create_uniform, ShaderHashProvider, ShaderHashProviderAny, ShaderPassBuilder,
  UniformBufferDataView, GPU,
};
use wgsl_shader_derives::wgsl_fn;

pub struct GridGround {
  grid_config: SceneItemRef<GridGroundConfig>,
}

impl PassContentWithCamera for &mut GridGround {
  fn render(
    &mut self,
    pass: &mut rendiation_scene_webgpu::SceneRenderPass,
    camera: &rendiation_scene_core::SceneCamera,
  ) {
    let base = pass.default_dispatcher();

    let impls: &mut IdentityMapper<InfinityShaderPlane, GridGroundConfig> = pass
      .resources
      .custom_storage
      .entry()
      .or_insert_with(Default::default);

    let implementation = impls.get_update_or_insert_with(
      &self.grid_config.read(),
      |grid_config| create_grid_gpu(*grid_config, pass.ctx.gpu),
      |gpu, grid_config| *gpu = create_grid_gpu(*grid_config, pass.ctx.gpu),
    );

    let camera_gpu = pass
      .resources
      .cameras
      .check_update_gpu(camera, pass.ctx.gpu);

    let effect = InfinityShaderPlaneEffect {
      plane: &implementation.plane,
      camera: camera_gpu,
    };

    let components: [&dyn RenderComponentAny; 3] =
      [&base, &effect, implementation.shading.as_ref()];
    RenderEmitter::new(components.as_slice()).render(&mut pass.ctx, &effect);
  }
}

fn create_grid_gpu(source: GridGroundConfig, gpu: &GPU) -> InfinityShaderPlane {
  InfinityShaderPlane {
    plane: create_uniform(
      ShaderPlane {
        normal: Vec3::new(0., 1., 0.),
        constant: 0.,
        ..Zeroable::zeroed()
      },
      gpu,
    ),
    shading: Box::new(GridGroundShading {
      shading: create_uniform(source, gpu),
    }),
  }
}

impl Default for GridGround {
  fn default() -> Self {
    Self {
      grid_config: SceneItemRef::new(GridGroundConfig {
        u_unit: 1.,
        v_unit: 1.,
        color: Vec4::splat(1.),
        ..Zeroable::zeroed()
      }),
    }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct GridGroundConfig {
  pub u_unit: f32,
  pub v_unit: f32,
  pub color: Vec4<f32>,
}

pub struct GridGroundShading {
  shading: UniformBufferDataView<GridGroundConfig>,
}
impl ShaderHashProvider for GridGroundShading {}
impl ShaderPassBuilder for GridGroundShading {
  fn setup_pass(&self, ctx: &mut webgpu::GPURenderPassCtx) {
    ctx.binding.bind(&self.shading, SB::Object);
  }
}
impl ShaderGraphProvider for GridGroundShading {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let shading = binding.uniform_by(&self.shading, SB::Object);
      let world_position = builder.query::<FragmentWorldPosition>()?;

      builder.register::<DefaultDisplay>(consts(Vec4::new(1., 1., 1., 0.5)));
      Ok(())
    })
  }
}

pub struct InfinityShaderPlane {
  plane: UniformBufferDataView<ShaderPlane>,
  shading: Box<dyn RenderComponentAny>,
}

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct)]
pub struct ShaderPlane {
  pub normal: Vec3<f32>,
  pub constant: f32,
}

pub struct InfinityShaderPlaneEffect<'a> {
  plane: &'a UniformBufferDataView<ShaderPlane>,
  camera: &'a CameraGPU,
}

impl<'a> DrawcallEmitter for InfinityShaderPlaneEffect<'a> {
  fn draw(&self, ctx: &mut webgpu::GPURenderPassCtx) {
    ctx.pass.draw(0..4, 0..1)
  }
}
impl<'a> ShaderHashProvider for InfinityShaderPlaneEffect<'a> {}
impl<'a> ShaderHashProviderAny for InfinityShaderPlaneEffect<'a> {
  fn hash_pipeline_and_with_type_id(&self, hasher: &mut webgpu::PipelineHasher) {
    self.plane.type_id().hash(hasher)
  }
}
impl<'a> ShaderPassBuilder for InfinityShaderPlaneEffect<'a> {
  fn setup_pass(&self, ctx: &mut webgpu::GPURenderPassCtx) {
    self.camera.setup_pass(ctx);
    ctx.binding.bind(self.plane, SB::Object);
  }
}

impl<'a> ShaderGraphProvider for InfinityShaderPlaneEffect<'a> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, _| {
      let out = generate_quad(builder.vertex_index).expand();
      builder.set_vertex_out::<FragmentUv>(out.uv);
      builder.register::<WorldVertexPosition>(out.position.xyz()); // too feed camera needs
      Ok(())
    })?;

    self.camera.build(builder)?;
    builder.log_result = true;
    builder.fragment(|builder, binding| {
      let proj = builder.query::<CameraProjectionMatrix>()?;
      let proj_inv = builder.query::<CameraProjectionInverseMatrix>()?;
      let view = builder.query::<CameraViewMatrix>()?;
      let view_inv = builder.query::<CameraWorldMatrix>()?;
      let uv = builder.query::<FragmentUv>()?;
      let plane = binding.uniform_by(self.plane, SB::Object);

      let unprojected =
        view_inv * proj_inv * (uv * consts(2.) - consts(Vec2::one()), 0., 1.).into();
      let unprojected = unprojected.xyz() / unprojected.w();

      let origin = view_inv.position();
      let direction = (unprojected - origin).normalize();

      let hit = ray_plane_intersect(origin, direction, plane);

      let plane_hit = hit.xyz();
      let plane_if_hit = hit.w(); // 1 is hit, 0 is not

      let plane_hit_project = proj * view * (plane_hit, consts(1.)).into();
      builder.set_explicit_depth(plane_hit_project.z() / plane_hit_project.w());

      builder.register::<FragmentWorldPosition>(plane_hit);
      builder.register::<IsHitInfinityPlane>(plane_if_hit);
      Ok(())
    })
  }

  // override
  fn post_build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, _| {
      let has_hit = builder.query::<IsHitInfinityPlane>()?;
      let previous_display = builder.query::<DefaultDisplay>()?;
      builder.register::<DefaultDisplay>((previous_display.xyz(), previous_display.w() * has_hit));
      Ok(())
    })
  }
}

both!(IsHitInfinityPlane, f32);

wgsl_fn! {
  fn ray_plane_intersect(origin: vec3<f32>, direction: vec3<f32>, plane: ShaderPlane) -> vec4<f32> {
    let denominator = dot(plane.normal, direction);

    // if denominator == T::zero() {
    //   // line is coplanar, return origin
    //   if plane.distance_to(&self.origin) == T::zero() {
    //     return T::zero().into();
    //   }

    //   return None;
    // }

    let t = -(dot(origin, plane.normal) + plane.constant) / denominator;

    if (t >= 0.0) {
      return vec4<f32>(origin + direction * t, 1.0);
    } else {
      return vec4<f32>(0.0);
    }
  }
}
