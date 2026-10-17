use crate::*;

pub const PLANE_DRAW_CMD: DrawCommand = FULL_SCREEN_TRIANGLE_DRAW_CMD;

pub fn ground_like_shader_plane() -> ShaderPlaneUniform {
  ShaderPlaneUniform::new(Vec3::new(0., 1., 0.), 0.)
}

both!(InfinityPlaneUnjitteredNDC, Vec2<f32>);

pub struct InfinityShaderPlaneEffect<'a> {
  pub plane: &'a UniformBufferCachedDataView<ShaderPlaneUniform>,
  pub camera: &'a dyn RenderComponent,
  pub reversed_depth: bool,
}

impl ShaderHashProvider for InfinityShaderPlaneEffect<'_> {
  shader_hash_type_id! {InfinityShaderPlaneEffect<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline(hasher);
    hasher.hash(self.reversed_depth);
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
      let out = generate_full_screen_triangle(builder.query::<VertexIndex>(), 0.).expand();
      builder.set_vertex_out::<FragmentUv>(out.uv);
      builder.register::<ClipPosition>(out.position);

      // the other geometries are jittered in clip space, so the ray of each pixel should be
      // constructed from the unjittered position to keep the plane content consistent with them.
      let ndc_xy = out.position.xy();
      let ndc_xy = if let Some(texel_size) = builder.try_query::<TexelSize>() {
        ndc_xy - texel_size * builder.query::<CameraJitter>()
      } else {
        ndc_xy
      };
      builder.set_vertex_out::<InfinityPlaneUnjitteredNDC>(ndc_xy);

      *builder.primitive_state() = PrimitiveState {
        topology: PrimitiveTopology::TriangleList,
        front_face: FrontFace::Cw,
        ..Default::default()
      };
    });

    builder.fragment(|builder, binding| {
      let camera_world_position = builder.query::<CameraWorldPositionHP>();
      let view_proj_none_translation = builder.query::<CameraViewNoneTranslationProjectionMatrix>();
      let view_proj_inv_none_translation =
        builder.query::<CameraViewNoneTranslationProjectionInverseMatrix>();

      let ndc_xy = builder.query::<InfinityPlaneUnjitteredNDC>();
      let plane = binding.bind_by(self.plane);

      let far = if self.reversed_depth {
        val(0.)
      } else {
        val(1.)
      };

      let far = view_proj_inv_none_translation * (ndc_xy, far, val(1.)).into();
      let far = far.xyz() / far.w();

      // we could skip this if we know the projection is perspective
      let near = if self.reversed_depth {
        val(1.)
      } else {
        val(0.)
      };
      let near = view_proj_inv_none_translation * (ndc_xy, near, val(1.)).into();
      let near = near.xyz() / near.w();

      let direction = (far - near).normalize();

      let render_space_plane =
        ShaderPlaneUniform::into_shader_plane(plane.load(), camera_world_position);
      let plane_normal = render_space_plane.normal;

      let hit_in_render_space = ray_plane_intersect(near, direction, render_space_plane);

      // use this if want draw content before the near plane
      // todo, what depth value should we write in this case??
      // let hit_in_render_space = hit_in_render_space.w().equals(0.).select_branched(
      //   || ray_plane_intersect(near, -direction, render_space_plane),
      //   || hit_in_render_space,
      // );

      // w is 1 if hit, 0 if not
      if_by(hit_in_render_space.w().equals(0.), || {
        builder.discard();
      });

      let plane_hit = hit_in_render_space.xyz();

      let plane_hit_project = view_proj_none_translation * (plane_hit, val(1.)).into();
      builder.register::<FragmentDepthOutput>(plane_hit_project.z() / plane_hit_project.w());

      builder.register::<FragmentRenderPosition>(plane_hit);
      builder.register::<FragmentRenderNormal>(plane_normal);
    })
  }
}
