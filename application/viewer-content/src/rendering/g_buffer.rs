use rendiation_shader_library::shader_uv_space_to_render_space;

use crate::*;

pub struct FrameGeometryBuffer {
  pub depth: RenderTargetView,
  pub normal: RenderTargetView,
  pub entity_id: Option<RenderTargetView>,
}

pub const MAX_U32_ID_BACKGROUND: rendiation_webgpu::Color = rendiation_webgpu::Color {
  r: u32::MAX as f64,
  g: 0.,
  b: 0.,
  a: 0.,
};

const ENABLE_MSAA_NORMAL_RESOLVE_DEPTH_AWARE_AVERAGE: bool = false;
const MSAA_NORMAL_RESOLVE_DEPTH_EPSILON: f32 = 1e-5;

impl FrameGeometryBuffer {
  pub fn should_skip_entity_id(cx: &mut FrameCtx) -> bool {
    let downgrade_info = &cx.gpu.info().downgrade_info;
    !downgrade_info
      .flags
      .contains(DownlevelFlags::INDEPENDENT_BLEND) // to support webgl!
  }

  pub fn new(cx: &mut FrameCtx, sample_count: u32) -> Self {
    Self {
      depth: depth_attachment().sample_count(sample_count).request(cx),
      normal: attachment()
        .format(TextureFormat::Rgba16Float)
        .sample_count(sample_count)
        .request(cx),
      entity_id: Self::should_skip_entity_id(cx).then(|| {
        attachment()
          .format(TextureFormat::R32Uint)
          .sample_count(sample_count)
          .request(cx)
      }),
    }
  }

  pub fn extend_pass_desc(
    &self,
    desc: &mut RenderPassDescription,
    depth_op: impl Into<Operations<f32>>,
    stencil_op: impl Into<Operations<u32>>,
  ) -> FrameGeometryBufferPassEncoder {
    desc.set_depth(&self.depth, depth_op, stencil_op);

    FrameGeometryBufferPassEncoder {
      normal: desc.push_color(&self.normal, clear_and_store(all_zero())),
      entity_id: self
        .entity_id
        .as_ref()
        .map(|entity_id| desc.push_color(entity_id, clear_and_store(MAX_U32_ID_BACKGROUND))),
    }
  }

  pub fn extend_pass_desc_for_subsequent_draw(
    &self,
    desc: &mut RenderPassDescription,
  ) -> FrameGeometryBufferPassEncoder {
    desc.set_depth(&self.depth, load_and_store(), load_and_store());

    FrameGeometryBufferPassEncoder {
      normal: desc.push_color(&self.normal, load_and_store()),
      entity_id: self
        .entity_id
        .as_ref()
        .map(|entity_id| desc.push_color(entity_id, load_and_store())),
    }
  }

  pub fn resolve_if_have_multi_sample(self, ctx: &mut FrameCtx, reverse_depth: bool) -> Self {
    if self.depth.sample_count() == 1 {
      return self;
    }

    let sample_count = self.depth.sample_count();
    let new_depth = depth_attachment().request(ctx);
    let new_normal = attachment().format(self.normal.format()).request(ctx);

    let new_entity_id = self.entity_id.as_ref().map(|entity_id| {
      attachment().format(entity_id.format()).request(ctx)
    });

    let resolver = MSAAGBufferResolver {
      depth: self.depth.expect_texture_view(),
      normal: self.normal.expect_texture_view(),
      entity_id: self.entity_id.map(|id| id.expect_texture_view()),
      sample_count,
      reverse_depth,
    };

    let mut pass_builder = pass("msaa resolve g buffer")
      .with_depth(&new_depth, load_and_store(), load_and_store())
      .with_color(&new_normal, store_full_frame());

    if let Some(ref target) = new_entity_id {
      pass_builder = pass_builder.with_color(target, clear_and_store(MAX_U32_ID_BACKGROUND));
    }

    pass_builder.render_ctx(ctx).by(&mut resolver.draw_quad());

    Self {
      depth: new_depth,
      normal: new_normal,
      entity_id: new_entity_id,
    }
  }
}

pub struct FrameGeometryBufferPassEncoder {
  pub normal: usize,
  pub entity_id: Option<usize>,
}

impl ShaderHashProvider for FrameGeometryBufferPassEncoder {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.normal.hash(hasher);
    self.entity_id.hash(hasher);
  }
}

impl ShaderPassBuilder for FrameGeometryBufferPassEncoder {}

impl GraphicsShaderProvider for FrameGeometryBufferPassEncoder {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      if let Some(entity_id) = self.entity_id {
        let id = builder.query_or_interpolate_by::<LogicalRenderEntityId, LogicalRenderEntityId>();
        builder.frag_output[entity_id].store(id);
      }

      let normal = builder.get_or_compute_fragment_normal();
      let out: Node<Vec4<f32>> = (normal, val(1.0)).into();
      builder.frag_output[self.normal].store(out);
    })
  }
}

impl ShaderPassBuilder for FrameGeometryBuffer {
  fn setup_pass(&self, cx: &mut GPURenderPassCtx) {
    self.normal.bind_pass(&mut cx.binding);
    self.depth.bind_pass(&mut cx.binding);
    if let Some(entity_id) = &self.entity_id {
      entity_id.bind_pass(&mut cx.binding);
    }
    cx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
  }
}

impl FrameGeometryBuffer {
  pub fn build_read_invocation(
    &self,
    binding: &mut ShaderBindGroupBuilder,
  ) -> FrameGeometryBufferReadInvocation {
    let normal = binding.bind_by(&self.normal);
    let input_size = normal.texture_dimension_2d(None).into_f32();

    FrameGeometryBufferReadInvocation {
      normal,
      depth: binding.bind_by(&DisableFiltering(&self.depth)),
      ids: binding.bind_by(&U32Texture2d),
      sampler: binding.bind_by(&DisableFiltering(ImmediateGPUSamplerViewBind)),
      input_size,
    }
  }
}

/// work around
pub struct U32Texture2d;
impl ShaderBindingProvider for U32Texture2d {
  type Node = ShaderBinding<ShaderTexture2DUint>;
  fn create_instance(&self, node: Node<Self::Node>) -> Self::ShaderInstance {
    node
  }
}

pub struct FrameGeometryBufferReadInvocation {
  pub depth: BindingNode<ShaderTexture2D>,
  pub normal: BindingNode<ShaderTexture2D>,
  pub ids: BindingNode<ShaderTexture2DUint>,
  pub sampler: BindingNode<ShaderSampler>,
  input_size: Node<Vec2<f32>>,
}

impl FrameGeometryBufferReadInvocation {
  pub fn read_depth_normal(&self, uv: Node<Vec2<f32>>) -> (Node<f32>, Node<Vec3<f32>>) {
    let depth = self.depth.sample(self.sampler, uv).x();
    let normal = self.normal.sample(self.sampler, uv).xyz().normalize();
    (depth, normal)
  }
  pub fn read_id(&self, uv: Node<Vec2<f32>>) -> Node<u32> {
    let u32_uv = (self.input_size * uv).floor().into_u32();
    self.ids.load_texel(u32_uv, 0).x()
  }
}

pub struct FrameGeometryBufferReconstructGeometryCtx<'a> {
  pub camera: &'a dyn RenderComponent,
  pub g_buffer: &'a FrameGeometryBuffer,
}
impl ShaderHashProvider for FrameGeometryBufferReconstructGeometryCtx<'_> {
  shader_hash_type_id! {FrameGeometryBufferReconstructGeometryCtx<'static>}
}
impl ShaderPassBuilder for FrameGeometryBufferReconstructGeometryCtx<'_> {
  fn setup_pass(&self, cx: &mut GPURenderPassCtx) {
    self.camera.setup_pass(cx);
    self.g_buffer.setup_pass(cx);
  }
}
impl GeometryCtxProvider for FrameGeometryBufferReconstructGeometryCtx<'_> {
  fn construct_ctx(
    &self,
    builder: &mut ShaderRenderPipelineBuilder,
  ) -> ENode<ShaderLightingGeometricCtx> {
    self.camera.build(builder);
    builder.fragment(|builder, binding| {
      let read = self.g_buffer.build_read_invocation(binding);
      let uv = builder.query::<FragmentUv>();
      let (depth, normal) = read.read_depth_normal(uv);
      let view_proj_inv = builder.query::<CameraViewNoneTranslationProjectionInverseMatrix>();
      let render_position = shader_uv_space_to_render_space(view_proj_inv, uv, depth);

      ENode::<ShaderLightingGeometricCtx> {
        position: render_position,
        normal,
        view_dir: -render_position.normalize(),
        camera_world_position: builder.query::<CameraWorldPositionHP>(),
        camera_world_none_translation_mat: builder.query::<CameraWorldNoneTranslationMatrix>(),
      }
    })
  }
}

struct MSAAGBufferResolver {
  depth: GPU2DMultiSampleDepthTextureView,
  normal: GPU2DMultiSampleTextureView,
  entity_id: Option<GPUTypedTextureView<TextureDimension2, MultiSampleOf<u32>>>,
  sample_count: u32,
  reverse_depth: bool,
}
impl ShaderHashProvider for MSAAGBufferResolver {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.sample_count.hash(hasher);
    self.reverse_depth.hash(hasher);
    self.entity_id.is_some().hash(hasher);
  }
}
impl ShaderPassBuilder for MSAAGBufferResolver {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.depth.bind_pass(&mut ctx.binding);
    self.normal.bind_pass(&mut ctx.binding);
    if let Some(entity_id) = &self.entity_id {
      entity_id.bind_pass(&mut ctx.binding);
    }
  }
}

impl GraphicsShaderProvider for MSAAGBufferResolver {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let frag_position = builder.query::<FragmentPosition>().xy().into_u32();
      let depth = binding.bind_by(&self.depth);
      let normal = binding.bind_by(&self.normal);
      let (sample_index, resolved_depth) = select_conservative_depth_sample(
        depth,
        frag_position,
        self.sample_count,
        self.reverse_depth,
      );

      let depth_stencil = builder.depth_stencil.as_mut().unwrap();
      depth_stencil.depth_compare = CompareFunction::Always;
      depth_stencil.depth_write_enabled = true;
      builder.register::<FragmentDepthOutput>(resolved_depth);

      let resolved = if ENABLE_MSAA_NORMAL_RESOLVE_DEPTH_AWARE_AVERAGE {
        resolve_normal_from_depth_cluster(
          depth,
          normal,
          frag_position,
          resolved_depth,
          self.sample_count,
        )
      } else {
        normal
          .load_texel_multi_sample_index(frag_position, sample_index)
          .xyz()
          .normalize()
      };

      builder.store_fragment_out_vec4f(0, (resolved, val(1.0)));

      if let Some(entity_id) = &self.entity_id {
        let id = binding.bind_by(entity_id);
        builder.store_fragment_out(
          1,
          id.load_texel_multi_sample_index(frag_position, sample_index).x(),
        );
      }
    });
  }
}

fn select_conservative_depth_sample(
  depth: BindingNode<ShaderMultiSampleDepthTexture2D>,
  frag_position: Node<Vec2<u32>>,
  sample_count: u32,
  reverse_depth: bool,
) -> (Node<u32>, Node<f32>) {
  let selected_sample_index = val(0_u32).make_local_var();
  let selected_depth = depth
    .load_texel_multi_sample_index(frag_position, val(0_u32))
    .make_local_var();

  for sample_index in 1..sample_count {
    let sample_depth = depth.load_texel_multi_sample_index(frag_position, val(sample_index));
    if_by(
      sample_depth.near_than(selected_depth.load(), reverse_depth),
      || {
        selected_depth.store(sample_depth);
        selected_sample_index.store(val(sample_index));
      },
    );
  }

  (selected_sample_index.load(), selected_depth.load())
}

fn resolve_normal_from_depth_cluster(
  depth: BindingNode<ShaderMultiSampleDepthTexture2D>,
  normal: BindingNode<ShaderMultiSampleTexture2D>,
  frag_position: Node<Vec2<u32>>,
  selected_depth: Node<f32>,
  sample_count: u32,
) -> Node<Vec3<f32>> {
  let normal_sum = val(Vec3::<f32>::zero()).make_local_var();
  let sample_hit_count = val(0.0).make_local_var();

  for sample_index in 0..sample_count {
    let sample_normal = normal
      .load_texel_multi_sample_index(frag_position, val(sample_index))
      .xyz();
    let sample_depth = depth.load_texel_multi_sample_index(frag_position, val(sample_index));
    let is_same_surface = (sample_depth - selected_depth)
      .abs()
      .less_equal_than(val(MSAA_NORMAL_RESOLVE_DEPTH_EPSILON));

    if_by(is_same_surface, || {
      normal_sum.store(normal_sum.load() + sample_normal);
      sample_hit_count.store(sample_hit_count.load() + val(1.0));
    });
  }

  (normal_sum.load() / sample_hit_count.load().splat()).normalize()
}
