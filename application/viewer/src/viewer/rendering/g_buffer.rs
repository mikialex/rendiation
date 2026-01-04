use rendiation_shader_library::shader_uv_space_to_render_space;

use crate::*;

pub struct FrameGeometryBuffer {
  pub depth: RenderTargetView,
  pub normal: RenderTargetView,
  pub entity_id: RenderTargetView,
  pub should_skip_entity_id: bool,
}

pub const MAX_U32_ID_BACKGROUND: rendiation_webgpu::Color = rendiation_webgpu::Color {
  r: u32::MAX as f64,
  g: 0.,
  b: 0.,
  a: 0.,
};

impl FrameGeometryBuffer {
  pub fn should_skip_entity_id(cx: &mut FrameCtx) -> bool {
    let downgrade_info = &cx.gpu.info().downgrade_info;
    !downgrade_info
      .flags
      .contains(DownlevelFlags::INDEPENDENT_BLEND) // to support webgl!
  }

  pub fn new(cx: &mut FrameCtx) -> Self {
    Self {
      depth: depth_attachment().request(cx),
      normal: attachment().format(TextureFormat::Rgba16Float).request(cx),
      entity_id: attachment().format(TextureFormat::R32Uint).request(cx),
      should_skip_entity_id: Self::should_skip_entity_id(cx),
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
      entity_id: if self.should_skip_entity_id {
        usize::MAX
      } else {
        desc.push_color(&self.entity_id, clear_and_store(MAX_U32_ID_BACKGROUND))
      },
    }
  }

  pub fn extend_pass_desc_for_subsequent_draw(
    &self,
    desc: &mut RenderPassDescription,
  ) -> FrameGeometryBufferPassEncoder {
    desc.set_depth(&self.depth, load_and_store(), load_and_store());

    FrameGeometryBufferPassEncoder {
      normal: desc.push_color(&self.normal, load_and_store()),
      entity_id: if self.should_skip_entity_id {
        usize::MAX
      } else {
        // although here the load store is same for all channels, we still need to reject the writing
        // as we never successfully write in first place
        desc.push_color(&self.entity_id, load_and_store())
      },
    }
  }
}

pub struct FrameGeometryBufferPassEncoder {
  pub normal: usize,
  pub entity_id: usize,
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
      if self.entity_id != usize::MAX {
        let id = builder.query_or_interpolate_by::<LogicalRenderEntityId, LogicalRenderEntityId>();
        builder.frag_output[self.entity_id].store(id);
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
    self.entity_id.bind_pass(&mut cx.binding);
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
    self.ids.load_texel(u32_uv, val(0)).x()
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
