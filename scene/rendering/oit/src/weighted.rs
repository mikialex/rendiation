use crate::*;

/// Implements Weighted, Blended Order-Independent Transparency,
/// from http://casual-effects.blogspot.de/2014/03/weighted-blended-order-independent.html.
/// This is an approximate order-independent transparency method.
/// The idea is that we assign each fragment an arbitrarily chosen weight,
/// here based on its depth, transparency, and color. Then we compute the
/// following quantities, where color0, color1, ... are premultiplied:
/// outColor: (weight0 * color0.rgba) + (weight1 * color1.rgba) + ...
///   (i.e. the weighted premultiplied sum, and)
/// outReveal: (1-color0.a) * (1-color1.a) * ...
///   (i.e. 1 minus the opacity of the result).
/// Then in the resolve pass, get the average weighted RGB color,
/// outColor.rgb/outColor.a, and blend it onto the image with the opacity
/// of the result. There's one more trick here; assuming it's being blended
/// onto an opaque surface, we can use the GL blend mode
/// GL_ONE_MINUS_SRC_ALPHA, GL_SRC_ALPHA
/// using outReveal (instead of 1-outReveal) as the alpha channel to blend
/// onto the image.
pub fn draw_weighted_oit(
  ctx: &mut FrameCtx,
  transparent_content: SceneModelRenderBatch,
  target_desc_without_final_color: RenderPassDescription,
  final_color_target: &RenderTargetView,
  scene_renderer: &dyn SceneRenderer,
  camera: &dyn RenderComponent,
  pass_com: &dyn RenderComponent,
  reverse_depth: bool,
) {
  let reveal_buffer = attachment().format(TextureFormat::R16Float).request(ctx);
  let accumulate_buffer = attachment().format(TextureFormat::Rgba16Float).request(ctx);

  let mut pass_target = target_desc_without_final_color.with_name("weighted_oit encode");

  let dispatch = DrawDispatch {
    reverse_depth,
    reveal_buffer_index: pass_target.push_color(&reveal_buffer, clear_and_store(color_same(1.))),
    accumulates_buffer_index: pass_target
      .push_color(&accumulate_buffer, clear_and_store(all_zero())),
  };
  let dispatch = &dispatch as &dyn RenderComponent;
  let pass_com = RenderArray([dispatch, pass_com]);

  let mut draw_content =
    scene_renderer.make_scene_batch_pass_content(transparent_content, camera, &pass_com, ctx);

  pass_target.render_ctx(ctx).by(&mut draw_content);

  pass("weighted_oit resolve")
    .with_color(final_color_target, load_and_store())
    .render_ctx(ctx)
    .by(
      &mut Composition {
        accumulates: accumulate_buffer,
        reveal: reveal_buffer,
      }
      .draw_quad(),
    );
}

struct DrawDispatch {
  reverse_depth: bool,
  reveal_buffer_index: usize,
  accumulates_buffer_index: usize,
}

impl ShaderHashProvider for DrawDispatch {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.reverse_depth.hash(hasher);
    self.reveal_buffer_index.hash(hasher);
    self.accumulates_buffer_index.hash(hasher);
  }
}

impl ShaderPassBuilder for DrawDispatch {}

impl GraphicsShaderProvider for DrawDispatch {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|cx, _| {
      let color_output = cx.query::<DefaultDisplay>();
      let color = color_output.xyz() * color_output.w(); // pre-multiply it

      let depth = cx.query::<FragmentPosition>().z();

      // Insert your favorite weighting function here. The color-based factor
      // avoids color pollution from the edges of wispy clouds. The z-based
      // factor gives precedence to nearer surfaces.

      // The depth functions in the paper want a camera-space depth of 0.1 < z < 500,
      // but the scene at the moment uses a range of about 0.01 to 50, so multiply
      // by 10 to get an adjusted depth:
      // todo, expose as a uniform

      let mut depth_z = depth * val(10.0);

      if self.reverse_depth {
        depth_z *= val(-1.0);
      }

      let dist_weight = (val(0.03) / (val(1e-5) + depth_z.pow(4.0))).clamp(1e-2, 3e3);

      let max_channel = color.max_channel().max(color_output.w());
      let alpha_weight = (max_channel * val(40.0) + val(0.01)).min(val(1.));
      let alpha_weight = alpha_weight * alpha_weight;

      let weight = alpha_weight * dist_weight;

      cx.store_fragment_out_vec4f(
        self.accumulates_buffer_index,
        vec4_node((color, color_output.w())) * weight,
      );
      cx.frag_output[self.accumulates_buffer_index].states.blend = Some(BlendState {
        color: BlendComponent {
          src_factor: BlendFactor::One,
          dst_factor: BlendFactor::One,
          operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
          src_factor: BlendFactor::One,
          dst_factor: BlendFactor::One,
          operation: BlendOperation::Add,
        },
      });

      cx.store_fragment_out_vec4f(
        self.reveal_buffer_index,
        vec4_node(color_output.w().splat()),
      );
      cx.frag_output[self.reveal_buffer_index].states.blend = Some(BlendState {
        color: BlendComponent {
          src_factor: BlendFactor::Zero,
          dst_factor: BlendFactor::OneMinusSrc,
          operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
          src_factor: BlendFactor::Zero,
          dst_factor: BlendFactor::OneMinusSrcAlpha,
          operation: BlendOperation::Add,
        },
      });

      cx.depth_stencil.as_mut().unwrap().depth_write_enabled = false;
    });
  }
}

struct Composition {
  accumulates: RenderTargetView,
  reveal: RenderTargetView,
}

impl ShaderHashProvider for Composition {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for Composition {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.accumulates);
    ctx.binding.bind(&self.reveal);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
  }
}

impl GraphicsShaderProvider for Composition {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|cx, binding| {
      let accumulates = binding.bind_by(&self.accumulates);
      let reveal = binding.bind_by(&self.reveal);
      let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

      let uv = cx.query::<FragmentUv>();
      let accumulates = accumulates.sample(sampler, uv);
      let reveal = reveal.sample(sampler, uv).x();

      let color = accumulates.xyz() / accumulates.w().max(1e-5).splat();

      cx.store_fragment_out_vec4f(0, vec4_node((color, reveal)));

      cx.frag_output[0].states.blend = Some(BlendState {
        color: BlendComponent {
          src_factor: BlendFactor::OneMinusSrcAlpha,
          dst_factor: BlendFactor::SrcAlpha,
          operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
          src_factor: BlendFactor::OneMinusSrcAlpha,
          dst_factor: BlendFactor::SrcAlpha,
          operation: BlendOperation::Add,
        },
      });
    })
  }
}
