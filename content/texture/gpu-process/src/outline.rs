use crate::*;

pub struct OutlineSource {
  pub position: Node<Vec3<f32>>,
  pub normal: Node<Vec3<f32>>,
  pub entity_id: Node<u32>,
}

pub trait OutlineComputeSourceInvocation {
  fn get_source(&self, uv: Node<Vec2<f32>>) -> OutlineSource;
}

pub trait OutlineComputeSourceProvider: ShaderHashProvider {
  fn build(&self, bind: &mut ShaderBindGroupBuilder) -> Box<dyn OutlineComputeSourceInvocation>;
  fn bind(&self, cx: &mut GPURenderPassCtx);
}

pub struct OutlineComputer<'a> {
  pub source: &'a dyn OutlineComputeSourceProvider,
}

impl ShaderHashProvider for OutlineComputer<'_> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.source.hash_pipeline(hasher);
  }

  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    self.source.hash_type_info(hasher);
  }
}

impl ShaderPassBuilder for OutlineComputer<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.source.bind(ctx);
  }
}

impl GraphicsShaderProvider for OutlineComputer<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let source = self.source.build(binding);

      let uv = builder.query::<FragmentUv>();
      let texel = builder.query::<TexelSize>();

      let top_sample_uv = uv + val(Vec2::new(0., 1.)) * texel;
      let bottom_sample_uv = uv + val(Vec2::new(0., -1.)) * texel;
      let left_sample_uv = uv + val(Vec2::new(-1., 0.)) * texel;
      let right_sample_uv = uv + val(Vec2::new(1., 0.)) * texel;

      let edge = val(0.);

      let center = source.get_source(uv);
      let top = source.get_source(top_sample_uv);
      let bottom = source.get_source(bottom_sample_uv);
      let left = source.get_source(left_sample_uv);
      let right = source.get_source(right_sample_uv);

      let top_to_center_dir = (center.position - top.position).normalize();
      let top_diff = top_to_center_dir.dot(top.normal).abs();

      let bottom_to_center_dir = (center.position - bottom.position).normalize();
      let bottom_diff = bottom_to_center_dir.dot(bottom.normal).abs();

      let left_to_center_dir = (center.position - left.position).normalize();
      let left_diff = left_to_center_dir.dot(left.normal).abs();

      let right_to_center_dir = (center.position - right.position).normalize();
      let right_diff = right_to_center_dir.dot(right.normal).abs();

      let shape_edge_ratio = (top_diff - bottom_diff)
        .abs()
        .max((left_diff - right_diff).abs());
      let shape_edge_ratio = shape_edge_ratio.smoothstep(0.15, 1.0);
      let edge = edge.max(shape_edge_ratio);

      let entity_top_diff = (top.entity_id - center.entity_id).abs();
      let entity_bottom_diff = (bottom.entity_id - center.entity_id).abs();
      let entity_left_diff = (left.entity_id - center.entity_id).abs();
      let entity_right_diff = (right.entity_id - center.entity_id).abs();
      let entity_edge_ratio = entity_top_diff
        .max(entity_bottom_diff)
        .max(entity_left_diff)
        .max(entity_right_diff)
        .min(1)
        .into_f32();

      let edge = edge.max(entity_edge_ratio);

      let top_normal_diff = val(1.) - center.normal.dot(top.normal);
      let bottom_normal_diff = val(1.) - center.normal.dot(bottom.normal);
      let left_normal_diff = val(1.) - center.normal.dot(left.normal);
      let right_normal_diff = val(1.) - center.normal.dot(right.normal);
      let normal_diff = top_normal_diff + bottom_normal_diff + left_normal_diff + right_normal_diff;

      let normal_bias = val(0.01);
      let normal_edge_ratio = (normal_diff - normal_bias).saturate();
      let edge = edge.max(normal_edge_ratio);

      builder.store_fragment_out_vec4f(0, (edge.splat(), val(1.)));
    })
  }
}
