use crate::*;

pub struct FXAA<'a> {
  pub source: &'a RenderTargetView,
}

impl ShaderHashProvider for FXAA<'_> {
  shader_hash_type_id! {FXAA<'static>}
}
impl GraphicsShaderProvider for FXAA<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let source = binding.bind_by(&self.source);
      let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

      let uv = builder.query::<FragmentUv>();
      let texel_size = builder.query::<TexelSize>();

      let output = fxaa(source, sampler, uv, texel_size);

      builder.store_fragment_out_vec4f(0, (output, val(1.)));
    })
  }
}
impl ShaderPassBuilder for FXAA<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.source);
    ctx.bind_immediate_sampler(&TextureSampler::default().with_double_linear().into_gpu());
  }
}

/// FXAA algorithm from NVIDIA, C# implementation by Jasper Flick, GLSL port by Dave Hoskins
/// http://developer.download.nvidia.com/assets/gamedev/files/sdk/11/FXAA_WhitePaper.pdf
/// https://catlikecoding.com/unity/tutorials/advanced-rendering/fxaa/
pub fn fxaa(
  input: BindingNode<ShaderTexture2D>,
  sampler: BindingNode<ShaderSampler>,
  uv: Node<Vec2<f32>>,
  texel_size: Node<Vec2<f32>>,
) -> Node<Vec3<f32>> {
  let luminance = sample_luminance_neighborhood(input, sampler, uv, texel_size);
  let uv = uv.make_local_var();

  if_by(should_skip_pixel(&luminance).not(), || {
    let pixel_blend = determine_pixel_blend_factor(&luminance);
    let edge = determine_edge(&luminance, texel_size);
    let edge_blend =
      determine_edge_blend_factor(input, sampler, uv.load(), texel_size, &luminance, &edge);
    let final_blend = pixel_blend.max(edge_blend);

    if_by(edge.is_horizontal.into_bool(), || {
      uv.store(uv.load() + vec2_node((val(0.0), edge.pixel_step * final_blend)));
    })
    .else_by(|| {
      uv.store(uv.load() + vec2_node((edge.pixel_step * final_blend, val(0.0))));
    });
  });

  input.sample(sampler, uv.load()).xyz()
}

const EDGE_STEP_COUNT: u32 = 6;
const EDGE_GUESS: f32 = 8.0;
const EDGE_STEPS: [f32; EDGE_STEP_COUNT as usize] = [1.0, 1.5, 2.0, 2.0, 2.0, 4.0];

fn sample_luminance(
  input: BindingNode<ShaderTexture2D>,
  sampler: BindingNode<ShaderSampler>,
  uv: Node<Vec2<f32>>,
) -> Node<f32> {
  input
    .sample_zero_level(sampler, uv)
    .xyz()
    .dot(vec3(0.3, 0.59, 0.11))
}

fn sample_luminance_offset(
  input: BindingNode<ShaderTexture2D>,
  sampler: BindingNode<ShaderSampler>,
  uv: Node<Vec2<f32>>,
  texel_size: Node<Vec2<f32>>,
  offset: impl Into<Node<Vec2<i32>>>,
) -> Node<f32> {
  let uv = uv + texel_size * offset.into().into_f32();
  sample_luminance(input, sampler, uv)
}

#[derive(ShaderStruct, Clone, Copy)]
struct LuminanceData {
  pub m: f32,
  pub n: f32,
  pub e: f32,
  pub s: f32,
  pub w: f32,
  pub ne: f32,
  pub nw: f32,
  pub se: f32,
  pub sw: f32,
  pub highest: f32,
  pub lowest: f32,
  pub contrast: f32,
}

fn sample_luminance_neighborhood(
  input: BindingNode<ShaderTexture2D>,
  sampler: BindingNode<ShaderSampler>,
  uv: Node<Vec2<f32>>,
  texel_size: Node<Vec2<f32>>,
) -> ENode<LuminanceData> {
  let m = sample_luminance(input, sampler, uv);
  let n = sample_luminance_offset(input, sampler, uv, texel_size, vec2(0, 1));
  let e = sample_luminance_offset(input, sampler, uv, texel_size, vec2(1, 0));
  let s = sample_luminance_offset(input, sampler, uv, texel_size, vec2(0, -1));
  let w = sample_luminance_offset(input, sampler, uv, texel_size, vec2(-1, 0));
  let ne = sample_luminance_offset(input, sampler, uv, texel_size, vec2(1, 1));
  let nw = sample_luminance_offset(input, sampler, uv, texel_size, vec2(-1, 1));
  let se = sample_luminance_offset(input, sampler, uv, texel_size, vec2(1, -1));
  let sw = sample_luminance_offset(input, sampler, uv, texel_size, vec2(-1, -1));

  let highest = m
    .max(n)
    .max(e)
    .max(s)
    .max(w)
    .max(ne)
    .max(nw)
    .max(se)
    .max(sw);
  let lowest = m
    .min(n)
    .min(e)
    .min(s)
    .min(w)
    .min(ne)
    .min(nw)
    .min(se)
    .min(sw);
  let contrast = highest - lowest;

  ENode::<LuminanceData> {
    m,
    n,
    e,
    s,
    w,
    ne,
    nw,
    se,
    sw,
    highest,
    lowest,
    contrast,
  }
}

fn should_skip_pixel(l: &ENode<LuminanceData>) -> Node<bool> {
  let contrast_threshold = val(0.0312);
  let relative_threshold = val(0.063);

  let threshold = contrast_threshold.max(relative_threshold * l.highest);
  l.contrast.less_than(threshold)
}

fn determine_pixel_blend_factor(l: &ENode<LuminanceData>) -> Node<f32> {
  let subpixel_blending = val(1.0);

  // determine the average luminance of all adjacent neighbors. But because the diagonal neighbors
  // are spatially further away from the middle, they should matter less. We factor this into our
  // average by doubling the weights of the NESW neighbors, dividing the total by twelve instead of
  // eight. The result is akin to a tent filter and acts as a low-pass filter.
  //
  // neighbor weights:
  //
  // 1 2 1
  // 2 x 2
  // 1 2 1
  let f = val(2.0) * (l.n + l.e + l.s + l.w);
  let f = f + l.ne + l.nw + l.se + l.sw;
  let f = f * val(1.0 / 12.0);

  // find the contrast between the middle and this average, via their absolute difference.
  // The result has now become a high-pass filter.
  let f = (f - l.m).abs();

  // filter is normalized relative to the contrast of the NESW cross, via a division.
  // Clamp the result to a maximum of 1, as we might end up with larger values thanks
  // to the filter covering more pixels than the cross
  let f = (f / l.contrast).saturate();

  // The result is a rather harsh transition to use as a blend factor. Use the smoothstep
  // function to smooth it out, then square the result of that to slow it down.
  let blend_factor = f.smoothstep(0.0, 1.0);
  blend_factor * blend_factor * subpixel_blending
}

#[derive(ShaderStruct, Clone, Copy)]
struct EdgeData {
  pub is_horizontal: Bool,
  pub pixel_step: f32,
  pub opposite_luminance: f32,
  pub gradient: f32,
}

fn determine_edge(l: &ENode<LuminanceData>, tex_size: Node<Vec2<f32>>) -> ENode<EdgeData> {
  let horizontal = (l.n + l.s - val(2.0) * l.m).abs() * val(2.0)
    + (l.ne + l.se - val(2.0) * l.e).abs()
    + (l.nw + l.sw - val(2.0) * l.w).abs();

  let vertical = (l.e + l.w - val(2.0) * l.m).abs() * val(2.0)
    + (l.ne + l.nw - val(2.0) * l.n).abs()
    + (l.se + l.sw - val(2.0) * l.s).abs();

  let is_horizontal = horizontal.greater_equal_than(vertical);

  let p_luminance = is_horizontal.select(l.n, l.e);
  let n_luminance = is_horizontal.select(l.s, l.w);
  let p_gradient = (p_luminance - l.m).abs();
  let n_gradient = (n_luminance - l.m).abs();

  let pixel_step = is_horizontal.select(tex_size.y(), tex_size.x());

  let pn = p_gradient.less_than(n_gradient);

  ENode::<EdgeData> {
    is_horizontal: is_horizontal.into_big_bool(),
    pixel_step: pn.select(-pixel_step, pixel_step),
    opposite_luminance: pn.select(n_luminance, p_luminance),
    gradient: pn.select(n_gradient, p_gradient),
  }
}

fn determine_edge_blend_factor(
  input: BindingNode<ShaderTexture2D>,
  sampler: BindingNode<ShaderSampler>,
  uv: Node<Vec2<f32>>,
  texel_size: Node<Vec2<f32>>,
  l: &ENode<LuminanceData>,
  e: &ENode<EdgeData>,
) -> Node<f32> {
  let is_horizontal = e.is_horizontal.into_bool();
  let uv_edge = is_horizontal.select(
    vec2_node((uv.x(), uv.y() + e.pixel_step * val(0.5))),
    vec2_node((uv.x() + e.pixel_step * val(0.5), uv.y())),
  );

  let edge_step = is_horizontal.select(
    vec2_node((texel_size.x(), val(0.0))),
    vec2_node((val(0.0), texel_size.y())),
  );

  let edge_luminance = (l.m + e.opposite_luminance) * val(0.5);
  let gradient_threshold = e.gradient * val(0.25);

  let puv = uv_edge + edge_step * val(EDGE_STEPS[0]);

  let p_luminance_delta = sample_luminance(input, sampler, puv) - edge_luminance;

  let p_at_end = p_luminance_delta
    .abs()
    .greater_equal_than(gradient_threshold)
    .make_local_var();

  let puv = puv.make_local_var();
  let p_luminance_delta = p_luminance_delta.make_local_var();
  let i = val(1_u32).make_local_var();

  let edge_steps = Node::<[f32; 6]>::from_array(EDGE_STEPS);

  loop_by(|cx| {
    let should_break = i
      .load()
      .greater_equal_than(val(EDGE_STEP_COUNT))
      .or(p_at_end.load());

    if_by(should_break, || {
      cx.do_break();
    });

    puv.store(puv.load() + edge_step * edge_steps.index(i.load()));
    p_luminance_delta.store(sample_luminance(input, sampler, puv.load()) - edge_luminance);

    p_at_end.store(
      p_luminance_delta
        .load()
        .abs()
        .greater_equal_than(gradient_threshold),
    );

    i.store(i.load() + val(1));
  });

  if_by(p_at_end.load().not(), || {
    puv.store(puv.load() + edge_step * val(EDGE_GUESS));
  });

  let nuv = uv_edge - edge_step * val(EDGE_STEPS[0]);
  let n_luminance_delta = sample_luminance(input, sampler, nuv) - edge_luminance;

  let n_at_end = n_luminance_delta
    .abs()
    .greater_equal_than(gradient_threshold)
    .make_local_var();

  let nuv = nuv.make_local_var();
  let n_luminance_delta = n_luminance_delta.make_local_var();
  let i = val(1_u32).make_local_var();

  loop_by(|cx| {
    let should_break = i
      .load()
      .greater_equal_than(val(EDGE_STEP_COUNT))
      .or(n_at_end.load());

    if_by(should_break, || {
      cx.do_break();
    });

    nuv.store(nuv.load() - edge_step * edge_steps.index(i.load()));
    n_luminance_delta.store(sample_luminance(input, sampler, nuv.load()) - edge_luminance);

    n_at_end.store(
      n_luminance_delta
        .load()
        .abs()
        .greater_equal_than(gradient_threshold),
    );

    i.store(i.load() + val(1));
  });

  if_by(n_at_end.load().not(), || {
    nuv.store(nuv.load() - edge_step * val(EDGE_GUESS));
  });

  let puv = puv.load();
  let nuv = nuv.load();
  let p_distance = is_horizontal.select(puv.x() - uv.x(), puv.y() - uv.y());
  let n_distance = is_horizontal.select(uv.x() - nuv.x(), uv.y() - nuv.y());

  let shortest_distance = p_distance
    .less_equal_than(n_distance)
    .select(p_distance, n_distance);
  let delta_sign = p_distance.less_equal_than(n_distance).select(
    p_luminance_delta.load().greater_equal_than(val(0.0)),
    n_luminance_delta.load().greater_equal_than(val(0.0)),
  );

  let r = val(0.).make_local_var();
  if_by(
    delta_sign.not_equals((l.m - edge_luminance).greater_equal_than(val(0.0))),
    || {
      r.store(val(0.5) - shortest_distance / (p_distance + n_distance));
    },
  );

  r.load()
}
