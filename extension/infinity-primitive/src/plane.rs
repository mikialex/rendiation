use std::hash::Hash;

use crate::*;

pub const PLANE_DRAW_CMD: DrawCommand = QUAD_DRAW_CMD;

pub fn ground_like_shader_plane() -> ShaderPlane {
  ShaderPlane::new(Vec3::new(0., 1., 0.), 0.)
}

pub struct InfinityShaderPlaneEffect<'a> {
  pub plane: &'a UniformBufferCachedDataView<ShaderPlane>,
  pub camera: &'a dyn RenderComponent,
  pub reversed_depth: bool,
}

impl ShaderHashProvider for InfinityShaderPlaneEffect<'_> {
  shader_hash_type_id! {InfinityShaderPlaneEffect<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline(hasher);
    self.reversed_depth.hash(hasher);
  }
}
impl ShaderPassBuilder for InfinityShaderPlaneEffect<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.camera.setup_pass(ctx);
    ctx.binding.bind(self.plane);
  }
}

impl GraphicsShaderProvider for InfinityShaderPlaneEffect<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.camera.build(builder);

    builder.vertex(|builder, _| {
      let out = generate_quad(builder.query::<VertexIndex>(), 0.).expand();
      builder.set_vertex_out::<FragmentUv>(out.uv);
      builder.register::<ClipPosition>((out.position.xyz(), val(1.)));

      builder.primitive_state = PrimitiveState {
        topology: PrimitiveTopology::TriangleStrip,
        front_face: FrontFace::Cw,
        ..Default::default()
      };
    });

    builder.fragment(|builder, binding| {
      let proj = builder.query::<CameraProjectionMatrix>();
      let world = builder.query::<CameraWorldMatrix>();
      let view = builder.query::<CameraViewMatrix>();
      let view_proj_inv = builder.query::<CameraViewProjectionInverseMatrix>();

      let uv = builder.query::<FragmentUv>();
      let plane = binding.bind_by(self.plane);

      let ndc_xy = uv * val(2.) - val(Vec2::one());
      let ndc_xy = ndc_xy * val(Vec2::new(1., -1.));

      let far = if self.reversed_depth {
        val(0.)
      } else {
        val(1.)
      };
      let near = if self.reversed_depth {
        val(1.)
      } else {
        val(0.)
      };

      let far = view_proj_inv * (ndc_xy, far, val(1.)).into();
      let near = view_proj_inv * (ndc_xy, near, val(1.)).into();

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
    })
  }

  // override
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let has_hit = builder.query::<IsHitInfinityPlane>();
      let previous_display = builder.query::<DefaultDisplay>();
      builder.register::<DefaultDisplay>((
        previous_display.xyz() * has_hit,
        previous_display.w() * has_hit,
      ));
    })
  }
}

both!(IsHitInfinityPlane, f32);
