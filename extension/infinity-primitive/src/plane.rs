use rendiation_shader_library::sampling::tbn;

use crate::*;

/// the plane is covered by three triangles, each triangle has one vertex at the plane point
/// nearest to the camera and two vertices at infinity, see [InfinityShaderPlaneEffect]
pub const PLANE_DRAW_CMD: DrawCommand = DrawCommand::Array {
  vertices: 0..9,
  instances: 0..1,
};

pub fn ground_like_shader_plane() -> ShaderPlaneUniform {
  ShaderPlaneUniform::new(Vec3::new(0., 1., 0.), 0.)
}

both!(InfinityPlaneHomogeneousRenderPosition, Vec4<f32>);
both!(InfinityPlaneRenderNormal, Vec3<f32>);

/// draw the infinite plane by real geometry in homogeneous space. the triangle composed by a
/// finite point (p, 1) and two points at infinity (d1, 0), (d2, 0) covers exactly the wedge
/// p + s * d1 + t * d2 (s, t >= 0), so three wedges cover the whole plane. the hardware does the
/// clipping, depth interpolation and early depth test, the fragment only does a division.
pub struct InfinityShaderPlaneEffect<'a> {
  pub plane: &'a UniformBufferCachedDataView<ShaderPlaneUniform>,
  pub camera: &'a dyn RenderComponent,
}

impl ShaderHashProvider for InfinityShaderPlaneEffect<'_> {
  shader_hash_type_id! {InfinityShaderPlaneEffect<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline(hasher);
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

    builder.vertex(|builder, binding| {
      let camera_world_position = builder.query::<CameraWorldPositionHP>();
      let plane = binding.bind_by(self.plane).load();
      let plane = ShaderPlaneUniform::into_shader_plane(plane, camera_world_position);

      let vertex_index = builder.query::<VertexIndex>();
      let triangle_index = vertex_index / val(3);
      let corner = vertex_index % val(3);

      // the camera is the origin of the render space, use the nearest point to keep precision
      let nearest_point = plane.normal * -plane.constant;

      let direction_index = corner
        .equals(2)
        .select(triangle_index + val(1), triangle_index);
      let angle = direction_index.into_f32() * val(std::f32::consts::TAU / 3.);
      let direction = tbn(plane.normal) * vec3_node((angle.cos(), angle.sin(), val(0.)));

      let position = corner
        .equals(0)
        .select((nearest_point, val(1.)), (direction, val(0.)));

      let clip_position = compute_jittered_clip_space_position(builder, position);
      builder.register::<ClipPosition>(clip_position);
      builder.set_vertex_out::<InfinityPlaneHomogeneousRenderPosition>(position);
      builder.set_vertex_out_with_given_interpolate::<InfinityPlaneRenderNormal>(
        plane.normal,
        ShaderInterpolation::Flat,
      );

      *builder.primitive_state() = PrimitiveState {
        topology: PrimitiveTopology::TriangleList,
        ..Default::default()
      };
    });

    builder.fragment(|builder, _| {
      // the perspective correct interpolation of the homogeneous position matches the clip
      // position interpolation, so the division gives the exact point on plane
      let position = builder.query::<InfinityPlaneHomogeneousRenderPosition>();
      let render_position = position.xyz() / position.w();

      builder.register::<FragmentRenderPosition>(render_position);
      let normal = builder.query::<InfinityPlaneRenderNormal>();
      builder.register::<FragmentRenderNormal>(normal);
    })
  }
}
