use __core::{any::Any, hash::Hash};
use futures::Stream;
use incremental::*;
use reactive::IncrementalListenBy;
use reactive::ReactiveMap;
use rendiation_scene_core::{
  any_change, GlobalIdReactiveSimpleMapping, IncrementalSignalPtr, IntoIncrementalSignalPtr,
};
use rendiation_scene_webgpu::{CameraGPU, PassContentWithSceneAndCamera, SceneRenderResourceGroup};
use rendiation_shader_api::*;
use webgpu::*;

use crate::MaterialStates;

pub struct GridGround {
  grid_config: IncrementalSignalPtr<GridGroundConfig>,
}

impl PassContentWithSceneAndCamera for &mut GridGround {
  fn render(
    &mut self,
    pass: &mut webgpu::FrameRenderPass,
    scene: &SceneRenderResourceGroup,
    camera: &rendiation_scene_core::SceneCamera,
  ) {
    let base = default_dispatcher(pass);

    let mut custom_storage = scene.resources.custom_storage.borrow_mut();
    let gpus: &mut ReactiveMap<IncrementalSignalPtr<GridGroundConfig>, InfinityShaderPlane> =
      custom_storage.entry().or_insert_with(Default::default);

    let grid_gpu = gpus.get_with_update(&self.grid_config, pass.ctx.gpu);

    let cameras = scene.scene_resources.cameras.read().unwrap();
    let camera_gpu = cameras.get_camera_gpu(camera).unwrap();

    let effect = InfinityShaderPlaneEffect {
      plane: &grid_gpu.plane,
      camera: camera_gpu,
    };

    let components: [&dyn RenderComponentAny; 3] = [&base, &effect, grid_gpu.shading.as_ref()];
    RenderEmitter::new(components.as_slice()).render(&mut pass.ctx, QUAD_DRAW_CMD);
  }
}

impl GlobalIdReactiveSimpleMapping<InfinityShaderPlane> for IncrementalSignalPtr<GridGroundConfig> {
  type ChangeStream = impl Stream<Item = ()> + Unpin;
  type Ctx<'a> = GPU;

  fn build(&self, ctx: &Self::Ctx<'_>) -> (InfinityShaderPlane, Self::ChangeStream) {
    let source = self.read();
    let grid_gpu = create_grid_gpu(*source, ctx);
    drop(source);

    let change = self.unbound_listen_by(any_change);
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
      .into_ptr(),
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
    ctx.binding.bind(&self.shading);
  }
}
impl GraphicsShaderProvider for GridGroundShading {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.fragment(|builder, binding| {
      let shading = binding.bind_by(&self.shading).load();
      let world_position = builder.query::<FragmentWorldPosition>()?;

      let grid = grid(world_position, shading);

      builder.register::<DefaultDisplay>(grid);
      Ok(())
    })
  }
}

#[shader_fn]
fn grid(position: Node<Vec3<f32>>, config: Node<GridGroundConfig>) -> Node<Vec4<f32>> {
  let coord = position.xz() * GridGroundConfig::scale(config);
  let grid =
    ((coord - val(Vec2::splat(0.5))).fract() - val(Vec2::splat(0.5))).abs() / coord.fwidth();
  let lined = grid.x().min(grid.y());
  (val(0.2), val(0.2), val(0.2), val(1.1) - lined.min(val(1.0))).into()
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

impl<'a> ShaderHashProvider for InfinityShaderPlaneEffect<'a> {}
impl<'a> ShaderHashProviderAny for InfinityShaderPlaneEffect<'a> {
  fn hash_pipeline_with_type_info(&self, hasher: &mut webgpu::PipelineHasher) {
    self.plane.type_id().hash(hasher)
  }
}
impl<'a> ShaderPassBuilder for InfinityShaderPlaneEffect<'a> {
  fn setup_pass(&self, ctx: &mut webgpu::GPURenderPassCtx) {
    self.camera.setup_pass(ctx);
    ctx.binding.bind(self.plane);
  }
}

impl<'a> GraphicsShaderProvider for InfinityShaderPlaneEffect<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.camera.inject_uniforms(builder);

    builder.vertex(|builder, _| {
      let out = generate_quad(builder.query::<VertexIndex>()?, 0.).expand();
      builder.set_vertex_out::<FragmentUv>(out.uv);
      builder.register::<ClipPosition>((out.position.xyz(), val(1.)));

      builder.primitive_state = webgpu::PrimitiveState {
        topology: webgpu::PrimitiveTopology::TriangleStrip,
        front_face: webgpu::FrontFace::Cw,
        ..Default::default()
      };

      Ok(())
    })?;

    builder.fragment(|builder, binding| {
      let proj = builder.query::<CameraProjectionMatrix>()?;
      let world = builder.query::<CameraWorldMatrix>()?;
      let view = builder.query::<CameraViewMatrix>()?;
      let view_proj_inv = builder.query::<CameraViewProjectionInverseMatrix>()?;

      let uv = builder.query::<FragmentUv>()?;
      let plane = binding.bind_by(self.plane);

      let ndc_xy = uv * val(2.) - val(Vec2::one());
      let ndc_xy = ndc_xy * val(Vec2::new(1., -1.));

      let far = view_proj_inv * (ndc_xy, val(1.), val(1.)).into();
      let near = view_proj_inv * (ndc_xy, val(0.), val(1.)).into();

      let far = far.xyz() / far.w().splat();
      let near = near.xyz() / near.w().splat();

      let direction = (far - near).normalize();
      let origin = near - (near - world.position()).dot(direction) * direction;

      let hit = ray_plane_intersect(origin, direction, plane.load().expand());

      let plane_hit = hit.xyz();
      let plane_if_hit = hit.w(); // 1 is hit, 0 is not

      let plane_hit_project = proj * view * (plane_hit, val(1.)).into();
      builder.register::<FragmentDepthOutput>(plane_hit_project.z() / plane_hit_project.w());

      builder.register::<FragmentWorldPosition>(plane_hit);
      builder.register::<IsHitInfinityPlane>(plane_if_hit);
      Ok(())
    })
  }

  // override
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
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

fn ray_plane_intersect(
  origin: Node<Vec3<f32>>,
  direction: Node<Vec3<f32>>,
  plane: ENode<ShaderPlane>,
) -> Node<Vec4<f32>> {
  let denominator = plane.normal.dot(direction); // I don't care if it's zero!

  let t = -(plane.normal.dot(origin) + plane.constant) / denominator;

  t.greater_equal_than(0.)
    .select((origin + direction * t, val(1.0)), Vec4::zero())
}
