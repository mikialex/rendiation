//! Draw tests: the fragment inputs of the trimming pipeline variant and the trimming itself
//! (plan-trim T2).

use std::time::Instant;

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
  let model = trimmed_random_model(11, 12, (3., 15.));
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

/// Draws `output` into an `Rgba32Float` target cleared to zero, without depth test, the
/// trimmed buckets run `trim`, the others write their shaded color with alpha 1.
async fn render_float(
  gpu: &GPU,
  camera: &Camera,
  output: &TessellationFrameOutput,
  trim: &dyn TessellationTrimFragment,
) -> Vec<Vec4<f32>> {
  let target = FloatTarget {
    format: TextureFormat::Rgba32Float,
    counting: false,
    depth: false,
  };
  let raw = render_target(gpu, camera, output, trim, &target).await;
  cast_slice(&raw).to_vec()
}

/// With depth test: `(shade, 0, 0, depth)` of the nearest fragment, depth `near / -z` (zero
/// for the background).
async fn render_depth(
  gpu: &GPU,
  camera: &Camera,
  output: &TessellationFrameOutput,
  trim: &dyn TessellationTrimFragment,
) -> Vec<Vec4<f32>> {
  let target = FloatTarget {
    format: TextureFormat::Rgba32Float,
    counting: false,
    depth: true,
  };
  let raw = render_target(gpu, camera, output, trim, &target).await;
  cast_slice(&raw).to_vec()
}

/// Fragment counts per pixel: `x` the fragments of the trimmed buckets (all of them when `trim`
/// writes alpha 0.5, the ones it keeps when it also discards), `y` the other fragments.
async fn render_fragment_counts(
  gpu: &GPU,
  camera: &Camera,
  output: &TessellationFrameOutput,
  trim: &dyn TessellationTrimFragment,
) -> Vec<Vec2<f32>> {
  let target = FloatTarget {
    format: TextureFormat::Rgba16Float,
    counting: true,
    depth: false,
  };
  let raw = render_target(gpu, camera, output, trim, &target).await;
  let half: &[u16] = cast_slice(&raw);
  half
    .chunks(4)
    .map(|c| Vec2::new(f16_to_f32(c[0]), f16_to_f32(c[1])))
    .collect()
}

/// Exact for the small integers of the fragment counts.
fn f16_to_f32(h: u16) -> f32 {
  let exponent = ((h >> 10) & 0x1f) as i32;
  let mantissa = (h & 0x3ff) as f32;
  if exponent == 0 {
    mantissa * 2f32.powi(-24)
  } else {
    (1. + mantissa / 1024.) * 2f32.powi(exponent - 15)
  }
}

async fn render_target(
  gpu: &GPU,
  camera: &Camera,
  output: &TessellationFrameOutput,
  trim: &dyn TessellationTrimFragment,
  float_target: &FloatTarget,
) -> Vec<u8> {
  let size = Size::from_u32_pair_min_one((camera.viewport.x as u32, camera.viewport.y as u32));
  let usage = TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;
  let target = create_empty_2d_texture_view(gpu, size, usage, float_target.format);

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
  let mut desc =
    pass("parameter test").with_color(&target.clone().into(), clear_and_store(all_zero()));
  if float_target.depth {
    let usage = TextureUsages::RENDER_ATTACHMENT;
    let depth = create_empty_2d_texture_view(gpu, size, usage, TextureFormat::Depth32Float);
    desc = desc.with_depth(&depth.into(), clear_and_store(0.), None);
  }
  let pass = encoder.begin_render_pass(&desc, None);
  let mut ctx = GPURenderPassCtx::new(pass, gpu.clone());
  render_tessellation_output(&mut ctx, output, &uniform, Some(trim), &[float_target], &[]);
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
  read.read_into_raw_unpadded_buffer()
}

/// Declares the single float target and writes `DefaultDisplay` to it. `counting` blends
/// additively and writes `(0, 1, 0, 0)` for the displays with alpha 1 (the untrimmed buckets),
/// `(1, 0, 0, 0)` for the others. `depth` tests and writes a `Depth32Float` attachment (nearest
/// is greater) and writes `(display.x, 0, 0, depth)`.
struct FloatTarget {
  format: TextureFormat,
  counting: bool,
  depth: bool,
}
impl ShaderHashProvider for FloatTarget {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.format);
    hasher.hash(self.counting);
    hasher.hash(self.depth);
  }
}
impl ShaderPassBuilder for FloatTarget {}
impl GraphicsShaderProvider for FloatTarget {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let mut target = channel(self.format);
      if self.counting {
        let add = BlendComponent {
          src_factor: BlendFactor::One,
          dst_factor: BlendFactor::One,
          operation: BlendOperation::Add,
        };
        target = target.with_blend(BlendState {
          color: add,
          alpha: add,
        });
      }
      builder.define_out_by(target);
      if self.depth {
        builder.depth_stencil = Some(DepthStencilState {
          format: TextureFormat::Depth32Float,
          depth_write_enabled: Some(true),
          depth_compare: Some(CompareFunction::Greater),
          stencil: Default::default(),
          bias: Default::default(),
        });
      }
    });
  }
  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      let display = builder.query::<DefaultDisplay>();
      let display = if self.counting {
        let untrimmed = display.w().equals(val(1.));
        untrimmed.select(
          val(Vec4::new(0., 1., 0., 0.)),
          val(Vec4::new(1., 0., 0., 0.)),
        )
      } else if self.depth {
        let depth = builder.query::<FragmentPosition>().z();
        (display.x(), val(0.), val(0.), depth).into()
      } else {
        display
      };
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
  let trim = TrimFragment::new(&gpu.device, trims, 0.);
  let trimmed = render_float(&gpu, &camera, t.output(), &trim).await;

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

/// The dilated trimming keeps every fragment inside the trim loops and only the fragments
/// outside within the dilation (plus a pixel for the pixel center and the distance estimate).
#[pollster::test]
async fn trim_dilation_keeps_near_fragments() {
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
  let dilation = 2.5;
  let mut t = GpuTessellator::with_trims(&gpu, &model, trims.clone(), Default::default());
  t.set_trim_dilation(dilation);
  let mut encoder = gpu.create_encoder();
  t.encode_frame(&gpu, &mut encoder, &camera);
  gpu.submit_encoder(encoder);
  assert_eq!(t.read_back(&gpu).await.stats.overflow, 0);

  let windings = render_float(&gpu, &camera, t.output(), &WindingOutput(trims.clone())).await;
  let trim = TrimFragment::new(&gpu.device, trims.clone(), 0.);
  let exact = render_float(&gpu, &camera, t.output(), &trim).await;
  trim.set_dilation(&gpu.queue, dilation);
  let dilated = render_float(&gpu, &camera, t.output(), &trim).await;

  let (width, height) = (camera.viewport.x as usize, camera.viewport.y as usize);
  let inside: Vec<_> = windings.iter().map(|p| p.z >= 2. && p.w != 0.).collect();
  let reach = dilation.ceil() as usize + 1;
  let (mut added, mut lost) = (0, 0);
  for index in 0..windings.len() {
    let (kept_exact, kept_dilated) = (exact[index].w != 0., dilated[index].w != 0.);
    lost += (kept_exact && !kept_dilated) as usize;
    if kept_dilated && !kept_exact {
      added += 1;
      let (x, y) = (index % width, index / width);
      let near_inside = (y.saturating_sub(reach)..(y + reach + 1).min(height)).any(|ny| {
        (x.saturating_sub(reach)..(x + reach + 1).min(width)).any(|nx| inside[ny * width + nx])
      });
      assert!(near_inside, "pixel ({x}, {y}) kept far from the trim");
    }
  }
  println!("dilation {dilation} px: {added} fragments added, {lost} lost");
  assert!(added > 1000);
  assert_eq!(lost, 0);
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

/// Writes alpha 0.5 (counted as a trimmed bucket fragment), after the discard of `trim` if any.
struct CountTrimmed(Option<TrimFragment>);
impl ShaderHashProvider for CountTrimmed {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.0.is_some());
  }
}
impl ShaderPassBuilder for CountTrimmed {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    if let Some(trim) = &self.0 {
      trim.setup_pass(ctx);
    }
  }
}
impl TessellationTrimFragment for CountTrimmed {
  fn build_trim(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupBuilder,
    input: TessellationFragmentInput,
  ) {
    if let Some(trim) = &self.0 {
      trim.build_trim(builder, binding, input);
    }
    builder.register::<DefaultDisplay>(val(Vec4::new(1., 0., 0., 0.5)));
  }
}

/// The fragments of the trimming pipeline variant on the steam controller model (skipped when
/// the file is missing), with and without trim classification: `cargo test --release -p
/// rendiation-parametric-rendering steam_trim_fragments -- --ignored --nocapture`.
#[pollster::test]
#[ignore]
async fn steam_trim_fragments() {
  let Ok(model) = ParametricNormalizedModel::load_pnm(super::tests_frame::STEAM_MODEL) else {
    println!("steam model not found, skipped");
    return;
  };
  let gpu = GPU::new(Default::default()).await.unwrap();
  let trims = TrimDeviceData::new(&gpu.device, &encode_model_trims(&model));
  for zoom in [1., 4., 20.] {
    let camera = fit_camera(&model, zoom);
    for classify in [false, true] {
      let mut t = GpuTessellator::with_trims(&gpu, &model, trims.clone(), Default::default());
      t.set_trim_classification(classify);
      let mut frame = None;
      for _ in 0..8 {
        let mut encoder = gpu.create_encoder();
        t.encode_frame(&gpu, &mut encoder, &camera);
        gpu.submit_encoder(encoder);
        let f = t.read_back(&gpu).await;
        gpu.poll(true);
        if f.stats.overflow == 0 {
          frame = Some(f);
          break;
        }
      }
      let frame = frame.expect("converged");
      let sum = |counts: &[Vec2<f32>]| {
        counts
          .iter()
          .fold((0., 0.), |(a, b), c| (a + c.x as f64, b + c.y as f64))
      };
      let all = render_fragment_counts(&gpu, &camera, t.output(), &CountTrimmed(None)).await;
      let kept = CountTrimmed(Some(TrimFragment::new(&gpu.device, trims.clone(), 0.)));
      let kept = render_fragment_counts(&gpu, &camera, t.output(), &kept).await;
      let ((trimmed, untrimmed), (kept, _)) = (sum(&all), sum(&kept));
      println!(
        "fit:{zoom} classify {classify}: {} leaves ({} trimmed) {} triangles | fragments {} of \
         which {:.1}% run the trimming ({:.1}% of those kept)",
        frame.stats.leaves,
        frame.stats.trimmed_leaves,
        frame.stats.triangles,
        trimmed + untrimmed,
        trimmed / (trimmed + untrimmed) * 100.,
        kept / trimmed * 100.,
      );
    }
  }
}

/// One or two pixel wide gaps: a pixel farther (by more than 2%, or the background) than both
/// pixels at distance 1 or 2 on a row or a column, with every pixel between them as far.
fn gap_pixels(depth: &[Vec4<f32>], width: usize, height: usize) -> Vec<bool> {
  let d = |x: usize, y: usize| depth[y * width + x].w;
  let mut gaps = vec![false; depth.len()];
  for y in 2..height - 2 {
    for x in 2..width - 2 {
      let here = d(x, y);
      let nearer = |n: f32| n > 0. && n > here * 1.02;
      for r in 1..=2 {
        for (dx, dy) in [(1, 0), (0, 1)] {
          let a = d(x - dx * r, y - dy * r);
          let b = d(x + dx * r, y + dy * r);
          let between_far = (1..r).all(|i| !nearer(d(x - dx * i, y - dy * i)))
            && (1..r).all(|i| !nearer(d(x + dx * i, y + dy * i)));
          if nearer(a) && nearer(b) && between_far {
            gaps[y * width + x] = true;
          }
        }
      }
    }
  }
  gaps
}

/// Writes the `size`² crop around `(cx, cy)`, magnified 4 times: gray shading, blue background,
/// red `marks`.
fn write_crop(
  path: &std::path::Path,
  depth: &[Vec4<f32>],
  marks: &[bool],
  width: usize,
  (cx, cy): (usize, usize),
  size: usize,
) {
  let height = depth.len() / width;
  let (x0, y0) = (
    cx.saturating_sub(size / 2).min(width - size),
    cy.saturating_sub(size / 2).min(height - size),
  );
  let image = image::RgbImage::from_fn(size as u32 * 4, size as u32 * 4, |x, y| {
    let index = (y0 + y as usize / 4) * width + x0 + x as usize / 4;
    let p = depth[index];
    if marks[index] {
      image::Rgb([255, 0, 0])
    } else if p.w > 0. {
      let g = (p.x.clamp(0., 1.) * 255.) as u8;
      image::Rgb([g, g, g])
    } else {
      image::Rgb([20, 30, 80])
    }
  });
  image.save(path).unwrap();
}

/// Cracks between neighbor faces on the steam controller model (skipped when the file is
/// missing) for several trim dilations: gap pixels, coverage growth, and with
/// `RENDIATION_TEST_OUTPUT_DIR` crops around the gaps: `cargo test --release -p
/// rendiation-parametric-rendering steam_trim_cracks -- --ignored --nocapture`.
#[pollster::test]
#[ignore]
async fn steam_trim_cracks() {
  let Ok(model) = ParametricNormalizedModel::load_pnm(super::tests_frame::STEAM_MODEL) else {
    println!("steam model not found, skipped");
    return;
  };
  let gpu = GPU::new(Default::default()).await.unwrap();
  let trims = TrimDeviceData::new(&gpu.device, &encode_model_trims(&model));
  let output_dir = std::env::var("RENDIATION_TEST_OUTPUT_DIR").ok();
  let dilations = [0., 0.5, 1., 1.5, 2.];
  for zoom in [1., 4., 20.] {
    let camera = fit_camera(&model, zoom);
    let (width, height) = (camera.viewport.x as usize, camera.viewport.y as usize);
    let mut images = Vec::new();
    for dilation in dilations {
      let mut t = GpuTessellator::with_trims(&gpu, &model, trims.clone(), Default::default());
      t.set_trim_dilation(dilation);
      for _ in 0..8 {
        let mut encoder = gpu.create_encoder();
        t.encode_frame(&gpu, &mut encoder, &camera);
        gpu.submit_encoder(encoder);
        let frame = t.read_back(&gpu).await;
        gpu.poll(true);
        if frame.stats.overflow == 0 {
          break;
        }
      }
      let trim = TrimFragment::new(&gpu.device, trims.clone(), dilation);
      images.push(render_depth(&gpu, &camera, t.output(), &trim).await);
    }
    let gaps: Vec<_> = images
      .iter()
      .map(|image| gap_pixels(image, width, height))
      .collect();
    let covered = |image: &[Vec4<f32>]| image.iter().filter(|p| p.w > 0.).count();
    let base = &images[0];
    for (i, dilation) in dilations.iter().enumerate() {
      let image = &images[i];
      // visible pixels already covered without dilation but showing another depth now
      let changed = image
        .iter()
        .zip(base)
        .filter(|(a, b)| b.w > 0. && (a.w - b.w).abs() > b.w * 1e-3)
        .count();
      let shade_changed = image
        .iter()
        .zip(base)
        .filter(|(a, b)| b.w > 0. && (a.w - b.w).abs() <= b.w * 1e-3 && (a.x - b.x).abs() > 0.05)
        .count();
      println!(
        "fit:{zoom} dilation {dilation}: {} gap pixels, {} covered ({:+}), {changed} covered \
         pixels changed depth, {shade_changed} changed shade at the same depth",
        gaps[i].iter().filter(|&&g| g).count(),
        covered(image),
        covered(image) as i64 - covered(base) as i64,
      );
    }
    if let Some(dir) = &output_dir {
      // crops around the first gaps fixed by the dilation of 1 px
      let fixed: Vec<_> = (0..base.len())
        .filter(|&i| gaps[0][i] && !gaps[2][i])
        .collect();
      let mut centers: Vec<(usize, usize)> = Vec::new();
      for &i in &fixed {
        let c = (i % width, i / width);
        if centers
          .iter()
          .all(|p| p.0.abs_diff(c.0) + p.1.abs_diff(c.1) > 100)
        {
          centers.push(c);
        }
        if centers.len() == 4 {
          break;
        }
      }
      for (n, &center) in centers.iter().enumerate() {
        for (i, dilation) in [(0, 0.), (2, 1.)] {
          let name = format!("crack_fit{zoom}_{n}_dilation{dilation}.png");
          let path = std::path::Path::new(dir).join(name);
          write_crop(&path, &images[i], &gaps[i], width, center, 48);
        }
      }
      println!(
        "fit:{zoom}: {} gap pixels fixed by 1 px, crops at {centers:?}",
        fixed.len()
      );
    }
  }
}

/// The time of drawing `output` once (with depth test, `trim` on the trimmed buckets) in ms:
/// the median over a few submissions of `DRAWS` draws each, divided by `DRAWS`.
fn time_draw(
  gpu: &GPU,
  camera: &Camera,
  output: &TessellationFrameOutput,
  trim: Option<&dyn TessellationTrimFragment>,
) -> f64 {
  let size = Size::from_u32_pair_min_one((camera.viewport.x as u32, camera.viewport.y as u32));
  let usage = TextureUsages::RENDER_ATTACHMENT;
  let target = create_empty_2d_texture_view(gpu, size, usage, TextureFormat::Rgba32Float);
  let depth = create_empty_2d_texture_view(gpu, size, usage, TextureFormat::Depth32Float);
  let float_target = FloatTarget {
    format: TextureFormat::Rgba32Float,
    counting: false,
    depth: true,
  };
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
  let uniform = TessellationDrawUniform::new(clip, Mat4::identity(), &Default::default(), output);
  let uniform = create_uniform(uniform, &gpu.device, "test draw uniform");
  const DRAWS: usize = 10;
  let mut times = Vec::new();
  for _ in 0..12 {
    let start = Instant::now();
    let mut encoder = gpu.create_encoder();
    for _ in 0..DRAWS {
      let desc = pass("draw timing")
        .with_color(&target.clone().into(), clear_and_store(all_zero()))
        .with_depth(&depth.clone().into(), clear_and_store(0.), None);
      let pass = encoder.begin_render_pass(&desc, None);
      let mut ctx = GPURenderPassCtx::new(pass, gpu.clone());
      render_tessellation_output(&mut ctx, output, &uniform, trim, &[&float_target], &[]);
    }
    gpu.submit_encoder(encoder);
    gpu.poll(true);
    times.push(start.elapsed().as_secs_f64() * 1e3 / DRAWS as f64);
  }
  // the first frames compile the pipelines
  let mut times = times[4..].to_vec();
  times.sort_by(f64::total_cmp);
  times[times.len() / 2]
}

/// Draw cost of the trimming on the steam controller model (skipped when the file is missing):
/// the trimmed draw with the band grid and with all segments (no acceleration) against the
/// whole cells, and the CPU replicated count of the segments a trimmed fragment visits, by the
/// segment count of its patch: `cargo test --release -p rendiation-parametric-rendering
/// steam_trim_draw_cost -- --ignored --nocapture`.
#[pollster::test]
#[ignore]
async fn steam_trim_draw_cost() {
  let Ok(model) = ParametricNormalizedModel::load_pnm(super::tests_frame::STEAM_MODEL) else {
    println!("steam model not found, skipped");
    return;
  };
  let gpu = GPU::new(Default::default()).await.unwrap();
  let config = TrimQuadraticConfig::for_model(&model);
  let band = encode_trims(&model, &config, &BandGrid::DEFAULT);
  let all = encode_trims(&model, &config, &AllSegments);
  let fine = BandGrid {
    max_band_log2: 6,
    ..BandGrid::DEFAULT
  };
  let fine_bands = encode_trims(&model, &config, &fine);
  let band_data = TrimDeviceData::new(&gpu.device, &band);
  let all_data = TrimDeviceData::new(&gpu.device, &all);
  let fine_data = TrimDeviceData::new(&gpu.device, &fine_bands);
  println!(
    "table words: 16 bands {}, 64 bands {}",
    band.table.len(),
    fine_bands.table.len()
  );
  for zoom in [1., 4., 20., 80.] {
    let camera = fit_camera(&model, zoom);
    let tessellate = |trims: &TrimDeviceData, classify: bool| {
      let mut t = GpuTessellator::with_trims(&gpu, &model, trims.clone(), Default::default());
      t.set_trim_classification(classify);
      t.set_trim_dilation(0.5);
      for _ in 0..8 {
        let mut encoder = gpu.create_encoder();
        t.encode_frame(&gpu, &mut encoder, &camera);
        gpu.submit_encoder(encoder);
        gpu.poll(true);
        let mut encoder = gpu.create_encoder();
        t.encode_frame(&gpu, &mut encoder, &camera);
        gpu.submit_encoder(encoder);
        gpu.poll(true);
        if t.last_stats().is_some_and(|s| s.overflow == 0) {
          break;
        }
      }
      t
    };
    let whole = tessellate(&band_data, false);
    let whole_ms = time_draw(&gpu, &camera, whole.output(), None);
    let t = tessellate(&band_data, true);
    let untrimmed_ms = time_draw(&gpu, &camera, t.output(), None);
    let fragment = TrimFragment::new(&gpu.device, band_data.clone(), 0.5);
    let band_ms = time_draw(&gpu, &camera, t.output(), Some(&fragment));
    fragment.set_dilation(&gpu.queue, 0.);
    let exact_ms = time_draw(&gpu, &camera, t.output(), Some(&fragment));
    let t_all = tessellate(&all_data, true);
    let all_fragment = TrimFragment::new(&gpu.device, all_data.clone(), 0.5);
    let all_ms = time_draw(&gpu, &camera, t_all.output(), Some(&all_fragment));
    let t_fine = tessellate(&fine_data, true);
    let fine_fragment = TrimFragment::new(&gpu.device, fine_data.clone(), 0.5);
    let fine_ms = time_draw(&gpu, &camera, t_fine.output(), Some(&fine_fragment));
    println!(
      "fit:{zoom}: draw whole cells {whole_ms:.2} ms, classified leaves without trimming \
       {untrimmed_ms:.2} ms, trimmed with band grid {band_ms:.2} ms (without dilation \
       {exact_ms:.2} ms, up to 64 bands {fine_ms:.2} ms), with all segments {all_ms:.2} ms"
    );

    // the visible trimmed fragments (last writer) and the segments their point query visits
    let windings = render_float(&gpu, &camera, t.output(), &WindingOutput(band_data.clone())).await;
    let (mut fragments, mut visits, mut constant) = (0usize, 0usize, 0usize);
    let mut fine_visits = 0usize;
    let mut by_count = [(0usize, 0usize); 4];
    let mut max_visits = 0;
    for p in windings.iter().filter(|p| p.z >= 2.) {
      let patch = p.z as usize - 2;
      let Some(view) = band.patch_view(patch) else {
        continue;
      };
      let uv = clamp_uv(Vec2::new(p.x, p.y));
      let mut n = 0;
      BandGrid::DEFAULT.point_query(view, uv, &mut |_| n += 1);
      let mut fine_n = 0;
      let fine_view = fine_bands.patch_view(patch).unwrap();
      fine.point_query(fine_view, uv, &mut |_| fine_n += 1);
      fine_visits += fine_n;
      fragments += 1;
      visits += n;
      constant += (n == 0) as usize;
      max_visits = max_visits.max(n);
      let class = match view.segments.len() {
        0..=8 => 0,
        9..=16 => 1,
        17..=32 => 2,
        _ => 3,
      };
      by_count[class].0 += 1;
      by_count[class].1 += n;
    }
    let per =
      |(f, v): (usize, usize)| format!("{f} fragments {:.2} visits", v as f64 / f.max(1) as f64);
    println!(
      "  {fragments} visible trimmed fragments: {:.2} segments visited on average (max \
       {max_visits}, {:.2} with up to 64 bands), {:.1}% in constant grid cells | by patch \
       segments <=8: {}, <=16: {}, <=32: {}, more: {}",
      visits as f64 / fragments.max(1) as f64,
      fine_visits as f64 / fragments.max(1) as f64,
      constant as f64 / fragments.max(1) as f64 * 100.,
      per(by_count[0]),
      per(by_count[1]),
      per(by_count[2]),
      per(by_count[3]),
    );
  }
}
