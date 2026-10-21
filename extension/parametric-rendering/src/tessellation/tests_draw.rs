//! Draw tests: the fragment inputs of the trimming pipeline variant and the trimming itself
//! (plan-trim T2).

use parametric_normalized_model::*;
use parametric_tessellation::test_scenes::trim::*;
use parametric_tessellation::test_scenes::*;
use parametric_tessellation::trim::*;
use parametric_tessellation::*;

use super::*;
use crate::*;

/// The parameter, the item and the patch received by the fragments of the trimmed buckets: the
/// surface point of the parameter must project to the pixel center up to the tessellation error.
#[pollster::test]
async fn trimmed_fragment_receives_exact_parameter() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let model = half_trimmed(random_model(11, 12, (3., 15.)));
  let camera = Camera {
    view: Mat4::identity(),
    focal_length: 200.,
    viewport: Vec2::new(400., 300.),
    near: 0.1,
  };
  let config = TessellationConfig::default();
  let mut t = GpuTessellator::new(&gpu, &model, config);
  let mut encoder = gpu.create_encoder();
  t.encode_frame(&gpu, &mut encoder, &camera);
  gpu.submit_encoder(encoder);
  let frame = t.read_back(&gpu).await;
  assert_eq!(frame.stats.overflow, 0);
  assert!(frame.stats.trimmed_leaves > 0 && frame.stats.trimmed_leaves < frame.stats.leaves);

  let pixels = render_float(&gpu, &camera, t.output(), &ParameterOutput).await;

  let items = camera_space_items(&model, &camera, |_| true).0;
  let (width, height) = (camera.viewport.x as usize, camera.viewport.y as usize);
  let (mut trimmed, mut untrimmed, mut max_offset) = (0, 0, 0f64);
  for (index, p) in pixels.iter().enumerate() {
    match p.w {
      0. => {}
      1. => untrimmed += 1,
      _ => {
        trimmed += 1;
        let item = &items[p.z as usize];
        assert_eq!(item.patch as f32, p.w - 2.);
        assert!(!model.patches[item.patch as usize].is_untrimmed());
        let s = item.surface.eval(p.x as f64, p.y as f64);
        let pixel = Vec2::new((index % width) as f64, (index / width) as f64) + Vec2::splat(0.5);
        let projected = Vec2::new(s.x, -s.y) * (camera.focal_length / -s.z) + camera.viewport * 0.5;
        max_offset = max_offset.max((projected - pixel).length());
      }
    }
  }
  println!("trimmed {trimmed} untrimmed {untrimmed} pixels, max offset {max_offset} px");
  assert!(trimmed > 1000 && untrimmed > 1000);
  assert!(max_offset < config.pixel_error + 0.05, "{max_offset}");
  assert_eq!(pixels.len(), width * height);
}

/// Every second patch gets a trim loop with an edge curve (the loop stays the patch boundary).
fn half_trimmed(mut model: ParametricNormalizedModel) -> ParametricNormalizedModel {
  for patch in model.patches.iter_mut().skip(1).step_by(2) {
    patch.loops[0].curves[0].source = TrimCurveSource::Edge {
      edge: EdgeId(0),
      reversed: false,
    };
  }
  model
}

/// Draws `output` into an `Rgba32Float` target cleared to zero, without depth test, the
/// trimmed buckets run `trim`, the others write their shaded color with alpha 1.
async fn render_float(
  gpu: &GPU,
  camera: &Camera,
  output: &TessellationFrameOutput,
  trim: &dyn TessellationTrimFragment,
) -> Vec<Vec4<f32>> {
  let size = Size::from_u32_pair_min_one((camera.viewport.x as u32, camera.viewport.y as u32));
  let usage = TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;
  let target = create_empty_2d_texture_view(gpu, size, usage, TextureFormat::Rgba32Float);

  // camera space to clip space, depth `near / -z`
  let (sx, sy) = (
    (camera.focal_length / (camera.viewport.x * 0.5)) as f32,
    (camera.focal_length / (camera.viewport.y * 0.5)) as f32,
  );
  #[rustfmt::skip]
  let clip = Mat4::new(
    sx, 0., 0., 0.,
    0., sy, 0., 0.,
    0., 0., 0., -1.,
    0., 0., camera.near as f32, 0.,
  );
  let settings = TessellationDrawSettings {
    trimmed_style: TrimmedPatchStyle::Trimmed,
    ..Default::default()
  };
  let uniform = TessellationDrawUniform::new(clip, Mat4::identity(), &settings, output);
  let uniform = create_uniform(uniform, &gpu.device, "test draw uniform");

  let mut encoder = gpu.create_encoder();
  let desc = pass("parameter test").with_color(&target.clone().into(), clear_and_store(all_zero()));
  let pass = encoder.begin_render_pass(&desc, None);
  let mut ctx = GPURenderPassCtx::new(pass, gpu.clone());
  render_tessellation_output(&mut ctx, output, &uniform, Some(trim), &[&FloatTarget], &[]);
  drop(ctx);
  let read = encoder.read_texture_2d(
    &gpu.device,
    &target,
    ReadRange {
      size,
      offset_x: 0,
      offset_y: 0,
    },
  );
  gpu.submit_encoder(encoder);
  let read = read.await.unwrap();
  cast_slice(&read.read_into_raw_unpadded_buffer()).to_vec()
}

/// Declares the single `Rgba32Float` target and writes `DefaultDisplay` to it.
struct FloatTarget;
impl ShaderHashProvider for FloatTarget {
  shader_hash_type_id! {}
}
impl ShaderPassBuilder for FloatTarget {}
impl GraphicsShaderProvider for FloatTarget {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      builder.define_out_by(channel(TextureFormat::Rgba32Float));
    });
  }
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let display = builder.query::<DefaultDisplay>();
      builder.store_fragment_out(0, display);
    });
  }
}

/// A trim fragment that only overrides the display with its inputs, `(u, v, item, patch + 2)`.
struct ParameterOutput;
impl ShaderHashProvider for ParameterOutput {
  shader_hash_type_id! {}
}
impl ShaderPassBuilder for ParameterOutput {}
impl TessellationTrimFragment for ParameterOutput {
  fn build_trim(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    _: &mut ShaderBindGroupBuilder,
    input: TessellationFragmentInput,
  ) {
    let tag = (input.patch + val(2)).into_f32();
    let display: Node<Vec4<f32>> = (input.uv, input.item.into_f32(), tag).into();
    builder.register::<DefaultDisplay>(display);
  }
}

/// Trimmed patches side by side facing the camera (no two cover the same pixel), the loops of
/// `trim_scenes` in turn.
fn trim_grid_model() -> ParametricNormalizedModel {
  let mut rng = TestRng(29);
  let mut builder = TestModelBuilder::default();
  let scenes = trim_scenes();
  for i in 0..10 {
    let face = builder.face([random_surface(&mut rng, i % 2 == 1)]);
    let (column, row) = ((i % 5) as f64, (i / 5) as f64);
    let t = Mat4::translate(((column - 2.) * 5., (row - 0.5) * 5., -30.));
    builder.instance(face, t, false);
    builder.model.patches[i].loops = scenes[i % scenes.len()].loops.clone();
  }
  builder.model
}

/// The fragment winding against the CPU f32 mirror at the fragment parameter, and the discarded
/// fragments against the winding of the original trim curves: they may only differ on the
/// pixels at the trim boundary.
#[pollster::test]
async fn trim_fragment_matches_cpu() {
  let gpu = GPU::new(Default::default()).await.unwrap();
  let model = trim_grid_model();
  let camera = Camera {
    view: Mat4::identity(),
    focal_length: 500.,
    viewport: Vec2::new(800., 600.),
    near: 0.1,
  };
  let encoding = encode_trims(&model, &trim_config(), &BandGrid::DEFAULT);
  let trims = TrimDeviceData::new(&gpu.device, &encoding);
  let mut t = GpuTessellator::with_trims(&gpu, &model, trims.clone(), Default::default());
  let mut encoder = gpu.create_encoder();
  t.encode_frame(&gpu, &mut encoder, &camera);
  gpu.submit_encoder(encoder);
  let frame = t.read_back(&gpu).await;
  assert_eq!(frame.stats.overflow, 0);

  let windings = render_float(&gpu, &camera, t.output(), &WindingOutput(trims.clone())).await;
  let trimmed = render_float(&gpu, &camera, t.output(), &TrimFragment { data: trims }).await;

  let references: Vec<_> = model
    .patches
    .iter()
    .map(|p| TrimReference::new(p, 1e-9))
    .collect();
  let (width, height) = (camera.viewport.x as usize, camera.viewport.y as usize);
  let expected: Vec<_> = windings
    .iter()
    .map(|p| {
      (p.z >= 2.).then(|| {
        let uv = clamp_uv(Vec2::new(p.x, p.y)).into_f64();
        references[p.z as usize - 2].winding(uv) != 0
      })
    })
    .collect();
  let (mut fragments, mut kernel_mismatched, mut inside, mut boundary_mismatched) = (0, 0, 0, 0);
  for (index, p) in windings.iter().enumerate() {
    let Some(expected_inside) = expected[index] else {
      continue;
    };
    fragments += 1;
    let (patch, uv) = (p.z as usize - 2, Vec2::new(p.x, p.y));
    if encoding.winding(patch, uv) != p.w as i32 {
      kernel_mismatched += 1;
    }
    let covered = trimmed[index].w != 0.;
    inside += covered as usize;
    if covered != expected_inside {
      // a mismatch must be next to a pixel of the other class
      let (x, y) = (index % width, index / width);
      let at_boundary = (y.saturating_sub(1)..(y + 2).min(height)).any(|ny| {
        (x.saturating_sub(1)..(x + 2).min(width))
          .any(|nx| expected[ny * width + nx].is_some_and(|e| e != expected_inside))
      });
      assert!(
        at_boundary,
        "pixel ({x}, {y}) covered {covered}, expected {expected_inside}"
      );
      boundary_mismatched += 1;
    }
  }
  println!(
    "{fragments} trimmed fragments, {inside} kept, {kernel_mismatched} fragment windings differ \
     from the cpu mirror, {boundary_mismatched} boundary pixels differ from the curves"
  );
  assert!(fragments > 20000 && inside > fragments / 4 && inside < fragments * 9 / 10);
  assert_eq!(kernel_mismatched, 0);
}

/// Writes `(u, v, patch + 2, winding)` without discarding.
struct WindingOutput(TrimDeviceData);
impl ShaderHashProvider for WindingOutput {
  shader_hash_type_id! {}
}
impl ShaderPassBuilder for WindingOutput {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.bind_pass(&mut ctx.binding);
  }
}
impl TessellationTrimFragment for WindingOutput {
  fn build_trim(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupBuilder,
    input: TessellationFragmentInput,
  ) {
    let trim = self.0.bind_shader(binding);
    let winding = trim.winding(input.patch, input.uv).into_f32();
    let tag = (input.patch + val(2)).into_f32();
    let display: Node<Vec4<f32>> = (input.uv, tag, winding).into();
    builder.register::<DefaultDisplay>(display);
  }
}
