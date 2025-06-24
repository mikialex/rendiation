use crate::*;

pub struct OitLoop32Renderer {
  layer_count: u32,
  cache: Option<OitLoop32RendererInstance>,
}

pub struct OitLoop32RendererInstance {
  depth: AtomicImageDowngrade,
  color: AtomicImageDowngrade,
  size: Size,
  layer_count: u32,
}

impl OitLoop32Renderer {
  pub fn new(layer_count: u32) -> Self {
    assert!(layer_count > 0);
    Self {
      cache: None,
      layer_count,
    }
  }

  pub fn get_renderer_instance(&mut self, size: Size, gpu: &GPU) -> &mut OitLoop32RendererInstance {
    if let Some(cache) = &mut self.cache {
      if cache.size != size || cache.layer_count != self.layer_count {
        self.cache = None;
      }
    }

    self.cache.get_or_insert_with(|| OitLoop32RendererInstance {
      depth: AtomicImageDowngrade::new(&gpu.device, size, self.layer_count),
      color: AtomicImageDowngrade::new(&gpu.device, size, self.layer_count),
      size,
      layer_count: self.layer_count,
    })
  }
}

impl OitLoop32RendererInstance {
  /// OIT_LOOP does not support MSAA at the moment.
  /// It uses two passes and a resolve pass; the first stores the depths of the
  /// front most OIT_LAYERS fragments per pixel in the A-buffer, in order from
  /// nearest to farthest. Then the second pass writes the sorted colors into
  /// another section of the A-buffer, and tail blends colors that didn't make it in.
  /// The resolve pass then blends the fragments from front to back.
  ///
  /// This relies on how for positive floating-point numbers x and y, x > y iff
  /// floatBitsToUint(x) > floatBitsToUint(y). As such, this depends on the
  /// viewport depths always being positive.
  ///
  /// The A-buffer is laid out like this:
  /// ```txt
  /// for each SSAA sample...
  ///   for each OIT layer...
  ///     for each pixel...
  ///       a r32ui depth value (via floatBitsToUint, cleared to background depth)
  ///     for each pixel...
  ///       a packed color in a uvec4
  /// ```
  pub fn draw_loop32_oit(
    &self,
    ctx: &mut FrameCtx,
    transparent_content: SceneModelRenderBatch,
    depth_base: &RenderTargetView,
    color_base: &RenderTargetView,
    scene_renderer: &dyn SceneRenderer<ContentKey = SceneContentKey>,
    camera: &dyn RenderComponent,
    pass_com: &dyn RenderComponent,
    reverse_depth: bool,
  ) {
    let far = if reverse_depth { 0_f32 } else { 1_f32 };
    self
      .depth
      .clear(&ctx.gpu.device, &mut ctx.encoder, far.to_bits());
    self.color.clear(&ctx.gpu.device, &mut ctx.encoder, 0);

    {
      let dispatch = Loop32DepthPrePass {
        oit_depth_layers: self.depth.clone(),
        reverse_depth,
      };
      let dispatch = &dispatch as &dyn RenderComponent;
      let mut draw_content = scene_renderer.make_scene_batch_pass_content(
        transparent_content.clone(),
        camera,
        &dispatch,
        ctx,
      );

      pass("loop32 oit depth pre pass")
        .with_depth(depth_base, load_and_store())
        .render_ctx(ctx)
        .by(&mut draw_content);
    }

    {
      let dispatch = OitColorPass {
        oit_depth_layers: self.depth.clone(),
        oit_color_layers: self.color.clone(),
        reverse_depth,
      };
      let dispatch = &dispatch as &dyn RenderComponent;
      let pass_com = RenderArray([dispatch, pass_com]);
      let mut draw_content =
        scene_renderer.make_scene_batch_pass_content(transparent_content, camera, &pass_com, ctx);
      pass("loop32 oit color pass")
        .with_color(color_base, load_and_store())
        .with_depth(depth_base, load_and_store())
        .render_ctx(ctx)
        .by(&mut draw_content);
    }

    pass("loop32 oit resolve pass")
      .with_color(color_base, load_and_store())
      .render_ctx(ctx)
      .by(
        &mut OitResolvePass {
          oit_depth_layers: self.depth.clone(),
          oit_color_layers: self.color.clone(),
          reverse_depth,
        }
        .draw_quad_with_blend(Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING)),
      );
  }
}

const USE_EARLY_DEPTH: bool = true;
const OIT_TAILBLEND: bool = true;

struct Loop32DepthPrePass {
  oit_depth_layers: AtomicImageDowngrade,
  reverse_depth: bool,
}

impl ShaderHashProvider for Loop32DepthPrePass {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.reverse_depth.hash(hasher);
  }
}
impl GraphicsShaderProvider for Loop32DepthPrePass {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|cx, binding| {
      cx.depth_stencil.as_mut().unwrap().depth_write_enabled = false;

      let oit_layers = self.oit_depth_layers.build(binding);
      let layer_count = oit_layers.layer_count();

      let depth = cx.query::<FragmentPosition>().z();
      let coord = cx.query::<FragmentPosition>().xy().into_u32();

      // Insert the floating-point depth (reinterpreted as a uint) into the list of depths
      let z_current = depth.bitcast::<u32>().make_local_var();
      let i = val(0_u32).make_local_var(); // Current position in the array

      if USE_EARLY_DEPTH {
        if self.reverse_depth {
          // Do some early tests to minimize the amount of insertion-sorting work we
          // have to do.
          // If the fragment is further away than the last depth fragment, skip it:
          let pretest = oit_layers.atomic_load(coord, layer_count - val(1));
          if_by(z_current.load().less_than(pretest), || {
            cx.discard();
          });
          // Check to see if the fragment can be inserted in the latter half of the
          // depth array:
          let pretest = oit_layers.atomic_load(coord, layer_count / val(2));
          if_by(z_current.load().less_than(pretest), || {
            i.store(layer_count / val(2));
          });
        } else {
          let pretest = oit_layers.atomic_load(coord, layer_count - val(1));
          if_by(z_current.load().greater_than(pretest), || {
            cx.discard();
          });
          let pretest = oit_layers.atomic_load(coord, layer_count / val(2));
          if_by(z_current.load().greater_than(pretest), || {
            i.store(layer_count / val(2));
          });
        }
      }

      // Try to insert z_current in the place of the first element of the array that
      // is greater than or equal to it. In the former case, shift all of the
      // remaining elements in the array down.
      ForRange::ranged((i.load(), layer_count).into()).for_each(|i, cx| {
        if self.reverse_depth {
          let z_test = oit_layers.atomic_max(coord, i, z_current.load());
          if_by(
            z_test
              .equals(val::<u32>(0_f32.to_bits()))
              .or(z_test.equals(z_current.load())),
            || {
              cx.do_break();
            },
          );
          z_current.store(z_test.min(z_current.load()));
        } else {
          let z_test = oit_layers.atomic_min(coord, i, z_current.load());
          if_by(
            z_test
              .equals(val::<u32>(1_f32.to_bits()))
              .or(z_test.equals(z_current.load())),
            || {
              cx.do_break();
            },
          );
          z_current.store(z_test.max(z_current.load()));
        }
      });
    })
  }
}
impl ShaderPassBuilder for Loop32DepthPrePass {
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.oit_depth_layers.bind(&mut ctx.binding);
  }
}

struct OitColorPass {
  oit_depth_layers: AtomicImageDowngrade,
  oit_color_layers: AtomicImageDowngrade,
  reverse_depth: bool,
}

impl ShaderHashProvider for OitColorPass {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.reverse_depth.hash(hasher);
  }
}
impl GraphicsShaderProvider for OitColorPass {
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|cx, binding| {
      let oit_depth_layers = self.oit_depth_layers.build(binding);
      let oit_color_layers = self.oit_color_layers.build(binding);
      let layer_count = oit_depth_layers.layer_count();

      // Get the un premultiplied linear-space RGBA color of this pixel
      let color_output = cx.query::<DefaultDisplay>();
      let srgb_color = shader_linear_to_srgb_convert(color_output.xyz());
      let srgb_color: Node<Vec4<f32>> = (srgb_color, color_output.w()).into();

      let z_current = cx.query::<FragmentPosition>().z().bitcast::<u32>();
      let coord = cx.query::<FragmentPosition>().xy().into_u32();

      let output_color = val(Vec4::<f32>::zero()).make_local_var();

      let skip = val(false).make_local_var();

      if USE_EARLY_DEPTH {
        // If this fragment was behind the front most OIT_LAYERS fragments, it didn't
        // make it in, so tail blend it:
        let depth = oit_depth_layers.atomic_load(coord, layer_count - val(1));
        let cond = if self.reverse_depth {
          depth.greater_than(z_current)
        } else {
          depth.less_than(z_current)
        };
        if_by(cond, || {
          skip.store(val(true));
          if OIT_TAILBLEND {
            // Premultiply alpha
            output_color.store(vec4_node((
              srgb_color.xyz() * srgb_color.w(),
              srgb_color.w(),
            )))
          } else {
            cx.discard();
          }
        });
      }

      if_by(skip.load().not(), || {
        // Use binary search to determine which index this depth value corresponds to
        // At each step, we know that it'll be in the closed interval [start, end].
        let start = val(0_u32).make_local_var();
        let end = (layer_count - val(1)).make_local_var();

        loop_by(|cx| {
          if_by(start.load().equals(end.load()), || cx.do_break());

          let mid = (start.load() + end.load()) / val(2);
          let z_test = oit_depth_layers.atomic_load(coord, mid);

          let cond = if self.reverse_depth {
            z_test.greater_than(z_current)
          } else {
            z_test.less_than(z_current)
          };

          if_by(cond, || {
            start.store(mid + val(1));
          })
          .else_by(|| {
            end.store(mid);
          });
        });

        // We now have start == end. Insert the packed color into the A-buffer at
        // this index.
        // todo, how to use store without atomic here? we can just use common texture?
        // https://github.com/gpuweb/gpuweb/issues/5071#issuecomment-2714533005
        oit_color_layers.atomic_store(coord, start.load(), srgb_color.pack4x8unorm());
      });

      cx.store_fragment_out_vec4f(0, output_color.load());
      cx.depth_stencil.as_mut().unwrap().depth_write_enabled = false;
      cx.frag_output[0].states.blend = Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING)
    })
  }
}
impl ShaderPassBuilder for OitColorPass {
  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.oit_depth_layers.bind(&mut ctx.binding);
    self.oit_color_layers.bind(&mut ctx.binding);
  }
}

#[shader_fn]
fn shader_linear_to_srgb_convert(srgb: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  (
    shader_linear_to_srgb_convert_per_channel(srgb.x()),
    shader_linear_to_srgb_convert_per_channel(srgb.y()),
    shader_linear_to_srgb_convert_per_channel(srgb.z()),
  )
    .into()
}

#[shader_fn]
fn shader_linear_to_srgb_convert_per_channel(c: Node<f32>) -> Node<f32> {
  c.less_than(0.0031308).select_branched(
    || c * val(12.92),
    || c.pow(1. / 2.4) * val(1.055) - val(0.055),
  )
}

struct OitResolvePass {
  oit_depth_layers: AtomicImageDowngrade,
  oit_color_layers: AtomicImageDowngrade,
  reverse_depth: bool,
}
impl ShaderHashProvider for OitResolvePass {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.reverse_depth.hash(hasher);
  }
}
impl GraphicsShaderProvider for OitResolvePass {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|cx, binding| {
      let oit_depth_layers = self.oit_depth_layers.build(binding);
      let oit_color_layers = self.oit_color_layers.build(binding);
      let layer_count = oit_depth_layers.layer_count();

      let out_color = val(Vec4::<f32>::zero()).make_local_var();
      let coord = cx.query::<FragmentPosition>().xy().into_u32();

      let background = if self.reverse_depth { val(0.) } else { val(1.) }.bitcast::<u32>();

      // Count the number of fragments for this pixel
      let fragments = val(0_u32).make_local_var();
      ForRange::ranged((val(0), layer_count).into()).for_each(|i, cx| {
        let depth = oit_depth_layers.atomic_load(coord, i);

        if_by(depth.not_equals(background), || {
          fragments.store(fragments.load() + val(1));
        })
        .else_by(|| {
          cx.do_break();
        });
      });

      ForRange::ranged((val(0), fragments.load()).into()).for_each(|i, _| {
        let packed_color = oit_color_layers.atomic_load(coord, i);
        out_color.store(do_blend_packed(out_color.load(), packed_color));
      });

      cx.store_fragment_out_vec4f(0, out_color.load());
    });
  }
}
impl ShaderPassBuilder for OitResolvePass {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.oit_depth_layers.bind(&mut ctx.binding);
    self.oit_color_layers.bind(&mut ctx.binding);
  }
}

// Sets color to the result of blending color over baseColor.
// Color and baseColor are both premultiplied colors.
fn do_blend(color: Node<Vec4<f32>>, base_color: Node<Vec4<f32>>) -> Node<Vec4<f32>> {
  let rgb = color.xyz() + base_color.xyz() * (val(1.) - color.w());
  let a = color.w() + base_color.w() * (val(1.) - color.w());
  (rgb, a).into()
}

// Sets color to the result of blending color over fragment.
// Color and fragment are both premultiplied colors; fragment
// is an rgba8 sRGB unpremultiplied color packed in a 32-bit uint.
fn do_blend_packed(color: Node<Vec4<f32>>, fragment: Node<u32>) -> Node<Vec4<f32>> {
  let unpacked = fragment.unpack4x8unorm();
  // Convert from unpremultiplied sRGB to premultiplied alpha
  let base_color = shader_linear_to_srgb_convert(unpacked.xyz()) * unpacked.w();
  let base_color = (base_color, unpacked.w()).into();
  do_blend(color, base_color)
}
