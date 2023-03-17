use __core::{any::Any, hash::Hash};
use futures::Stream;
use incremental::*;
use reactive::ReactiveMap;
use rendiation_scene_core::{
  any_change, IntoSceneItemRef, SceneItemReactiveSimpleMapping, SceneItemRef,
};
use rendiation_scene_webgpu::{generate_quad, CameraGPU, MaterialStates, PassContentWithCamera};
use shadergraph::*;
use webgpu::{
  create_uniform, DrawcallEmitter, RenderComponent, RenderComponentAny, RenderEmitter,
  ShaderHashProvider, ShaderHashProviderAny, ShaderPassBuilder, UniformBufferDataView, GPU,
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

    let gpus: &mut ReactiveMap<SceneItemRef<GridGroundConfig>, InfinityShaderPlane> = pass
      .resources
      .custom_storage
      .entry()
      .or_insert_with(Default::default);

    let grid_gpu = gpus.get_with_update(&self.grid_config, pass.ctx.gpu);
    let camera_gpu = pass.resources.cameras.get_with_update(camera, pass.ctx.gpu);

    let effect = InfinityShaderPlaneEffect {
      plane: &grid_gpu.plane,
      camera: camera_gpu,
    };

    let components: [&dyn RenderComponentAny; 3] = [&base, &effect, grid_gpu.shading.as_ref()];
    RenderEmitter::new(components.as_slice()).render(&mut pass.ctx, &effect);
  }
}

impl SceneItemReactiveSimpleMapping<InfinityShaderPlane> for SceneItemRef<GridGroundConfig> {
  type ChangeStream = impl Stream<Item = ()> + Unpin;
  type Ctx = GPU;

  fn build(&self, ctx: &Self::Ctx) -> (InfinityShaderPlane, Self::ChangeStream) {
    let source = self.read();
    let grid_gpu = create_grid_gpu(**source, ctx);

    let change = source.listen_by(any_change);
    (grid_gpu, change)
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
      grid_config: GridGroundConfig {
        scale: Vec2::one(),
        color: Vec4::splat(1.),
        ..Zeroable::zeroed()
      }
      .into_ref(),
    }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Incremental)]
pub struct GridGroundConfig {
  pub scale: Vec2<f32>,
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

      let grid = grid(world_position, shading);

      builder.register::<DefaultDisplay>(grid);
      Ok(())
    })
  }
}

wgsl_fn!(
  fn grid(position: vec3<f32>, config: GridGroundConfig) -> vec4<f32> {
    let coord = position.xz * config.scale;
    let grid = abs(fract(coord - 0.5) - 0.5) / fwidth(coord);
    let lined = min(grid.x, grid.y);
    return vec4<f32>(0.2, 0.2, 0.2, 1.0 - min(lined, 1.0) + 0.1);
  }
);

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
    self.camera.inject_uniforms(builder);

    builder.vertex(|builder, _| {
      let out = generate_quad(builder.query::<VertexIndex>()?).expand();
      builder.set_vertex_out::<FragmentUv>(out.uv);
      builder.register::<ClipPosition>((out.position.xyz(), 1.));

      builder.primitive_state = webgpu::PrimitiveState {
        topology: webgpu::PrimitiveTopology::TriangleStrip,
        front_face: webgpu::FrontFace::Cw,
        ..Default::default()
      };

      Ok(())
    })?;

    builder.fragment(|builder, binding| {
      let proj = builder.query::<CameraProjectionMatrix>()?;
      let proj_inv = builder.query::<CameraProjectionInverseMatrix>()?;
      let view = builder.query::<CameraViewMatrix>()?;
      let view_inv = builder.query::<CameraWorldMatrix>()?;

      let uv = builder.query::<FragmentUv>()?;
      let plane = binding.uniform_by(self.plane, SB::Object);

      let ndc_xy = uv * consts(2.) - consts(Vec2::one());
      let ndc_xy = ndc_xy * consts(Vec2::new(1., -1.));

      let unprojected = view_inv * proj_inv * (ndc_xy, 0., 1.).into();
      let unprojected = unprojected.xyz() / unprojected.w();

      let origin = view_inv.position();
      let direction = (unprojected - origin).normalize();

      let hit = ray_plane_intersect(origin, direction, plane);

      let plane_hit = hit.xyz();
      let plane_if_hit = hit.w(); // 1 is hit, 0 is not

      let plane_hit_project = proj * view * (plane_hit, consts(1.)).into();
      builder.register::<FragmentDepthOutput>(plane_hit_project.z() / plane_hit_project.w());

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
      builder.register::<DefaultDisplay>((
        previous_display.xyz() * has_hit,
        previous_display.w() * has_hit,
      ));

      MaterialStates {
        blend: webgpu::BlendState::ALPHA_BLENDING.into(),
        depth_write_enabled: false,
        depth_compare: webgpu::CompareFunction::LessEqual,
        ..Default::default()
      }
      .apply_pipeline_builder(builder);
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
