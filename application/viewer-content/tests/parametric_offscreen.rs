//! Offscreen smoke test of the parametric model rendering in the viewer frame: the model must
//! cover pixels in front of an empty scene, for the msaa and the taa frame paths.
//!
//! Uses a synthetic model by default, `RENDIATION_VIEWER_PNM` selects a `.pnm` file instead, and
//! `RENDIATION_TEST_OUTPUT_DIR` writes the rendered images there.

use parametric_normalized_model::*;
use rendiation_texture_core::Size;
use rendiation_viewer_content::*;
use rendiation_webgpu::*;

#[test]
fn parametric_model_renders_offscreen() {
  setup_global_database(Default::default());
  register_viewer_content_data_model();

  let model_source = || match std::env::var("RENDIATION_VIEWER_PNM") {
    Ok(path) => {
      ParametricModelSource::new(ParametricNormalizedModel::load_pnm(&path).unwrap(), path)
    }
    Err(_) => ParametricModelSource::new(synthetic_model(), "synthetic".into()),
  };

  let hidden_trimmed = ParametricRenderSettings {
    draw: TessellationDrawSettings {
      color_mode: TessellationColorMode::ByK,
      trimmed_style: TrimmedPatchStyle::Hidden,
      ..Default::default()
    },
    ..Default::default()
  };
  let cases = [
    ("msaa", true, false, ParametricRenderSettings::default()),
    ("taa", false, true, ParametricRenderSettings::default()),
    ("hidden_trimmed", false, false, hidden_trimmed),
  ];

  let mut coverages = Vec::new();
  for (name, msaa, taa, settings) in cases {
    let mut config = ViewerInitConfig {
      enable_msaa: msaa,
      enable_taa: taa,
      enable_grid_ground: false,
      enable_shadow: false,
      ..Default::default()
    };
    config.init_only.enable_reverse_z = true;

    let empty = render(&config, None);
    let with_model = render(&config, Some((model_source(), settings)));
    let covered = with_model
      .pixels
      .chunks(4)
      .zip(empty.pixels.chunks(4))
      .filter(|(a, b)| a != b)
      .count();
    let ratio = covered as f64 / (with_model.pixels.len() / 4) as f64;
    println!(
      "{name}: parametric model covers {:.1}% of the view, {:?}",
      ratio * 100.,
      with_model.stats
    );

    if let Ok(dir) = std::env::var("RENDIATION_TEST_OUTPUT_DIR") {
      let path = std::path::Path::new(&dir).join(format!("parametric_{name}.png"));
      with_model.write_png(&path);
    }

    let stats = with_model.stats.expect("tessellation finished");
    assert!(stats.leaves > 0 && stats.triangles > 0);
    assert!(ratio > 0.05, "{name}: coverage {ratio}");
    coverages.push(ratio);
  }
  assert!(
    coverages[2] < coverages[0],
    "hidden trimmed patches still drawn"
  );
}

const SIZE: (u32, u32) = (640, 480);

struct RenderResult {
  pixels: Vec<u8>,
  format: TextureFormat,
  stats: Option<ParametricTessellationStats>,
}

impl RenderResult {
  fn write_png(&self, path: &std::path::Path) {
    let row = SIZE.0 * 4;
    rendiation_texture_exporter::write_raw_gpu_buffer_image_as_png(
      &mut std::fs::File::create(path).unwrap(),
      Size::from_u32_pair_min_one(SIZE),
      &self.pixels,
      self.format,
      row,
      row,
    );
  }
}

/// Render frames until the tessellation is done (and the taa history settled), then read back.
fn render(
  config: &ViewerInitConfig,
  model: Option<(ParametricModelSource, ParametricRenderSettings)>,
) -> RenderResult {
  let gpu_config = config.make_gpu_platform_config().make_gpu_create_config();
  let gpu = pollster::block_on(GPU::new(gpu_config)).unwrap();
  let worker = TaskSpawner::new("parametric test", None);
  let mut viewer = Viewer::new(gpu.clone(), config, worker.clone());

  let scene = global_entity_of::<SceneEntity>()
    .entity_writer()
    .new_entity(|w| w);
  write_global_db_component::<SceneSolidBackground>().write(scene, Some(Vec3::new(0.05, 0.1, 0.3)));

  let (center, radius) = model
    .as_ref()
    .map(|(m, _)| {
      (
        m.bounds.center(),
        m.bounds.min.distance_to(m.bounds.center()),
      )
    })
    .unwrap_or((Vec3::zero(), 1.));
  let eye = center + Vec3::new(0.6, 0.5, 1.).normalize() * radius * 2.5;
  let camera_node = global_entity_of::<SceneNodeEntity>()
    .entity_writer()
    .new_entity(|w| {
      w.write::<SceneNodeLocalMatrixComponent>(&Mat4::lookat(eye, center, Vec3::new(0., 1., 0.)))
    });
  let projection = PerspectiveProjection {
    near: (radius * 0.01) as f32,
    far: (radius * 10.) as f32,
    aspect: SIZE.0 as f32 / SIZE.1 as f32,
    ..Default::default()
  };
  let camera = global_entity_of::<SceneCameraEntity>()
    .entity_writer()
    .new_entity(|w| {
      w.write::<SceneCameraPerspective>(&Some(projection))
        .write::<SceneCameraNode>(&camera_node.some_handle())
    });

  let surface_id = 0;
  viewer.surfaces_content.insert(
    surface_id,
    ViewerSurfaceContent {
      viewports: vec![ViewerViewPort {
        id: alloc_global_res_id(),
        viewport: Vec4::new(0., 0., SIZE.0 as f32, SIZE.1 as f32),
        camera,
        camera_node,
        debug_camera_for_view_related: None,
        scene,
      }],
      device_pixel_ratio: 1.,
    },
  );
  viewer.update_view_ty_immediate();

  let has_model = model.is_some();
  if let Some((model, settings)) = model {
    viewer.rendering.parametric.set_model(model);
    viewer.rendering.parametric.settings = settings;
  }

  let target = RenderTargetView::Texture(
    PooledTextureKey {
      size: Size::from_u32_pair_min_one(SIZE),
      format: TextureFormat::Rgba8UnormSrgb,
      sample_count: 1,
      require_mipmaps: false,
      usage: TextureUsages::RENDER_ATTACHMENT
        | TextureUsages::COPY_SRC
        | TextureUsages::TEXTURE_BINDING,
    }
    .create_directly(&gpu),
  );

  let mut data_scheduler = ViewerDataScheduler::new(None);
  let mut dyn_cx = DynCx::default();
  let mut settled_frames = 0;
  for _ in 0..2000 {
    unsafe { dyn_cx.register_cx::<ViewerDataScheduler>(&mut data_scheduler) };
    setup_new_frame_allocator(1024 * 1024);
    // keep every frame rendering, the result is read from the last one
    viewer.rendering_root.notify_change();
    viewer.draw_canvas(
      surface_id,
      &target,
      &worker,
      &mut data_scheduler,
      &mut dyn_cx,
      None,
      &mut (),
    );
    unsafe { dyn_cx.unregister_cx::<ViewerDataScheduler>() };

    if !has_model || viewer.rendering.parametric.stats().is_some() {
      settled_frames += 1;
      if settled_frames > 40 {
        break;
      }
    } else {
      std::thread::sleep(std::time::Duration::from_millis(10));
    }
  }

  let texture: GPU2DTextureView = target.expect_texture_view();
  let mut encoder = gpu.create_encoder();
  let read = encoder.read_texture_2d(
    &gpu.device,
    &texture,
    ReadRange {
      size: texture.size(),
      offset_x: 0,
      offset_y: 0,
    },
  );
  gpu.submit_encoder(encoder);
  let read = pollster::block_on(read).unwrap();

  let stats = viewer.rendering.parametric.stats().cloned();
  let mut drop_cx = DynCx::default();
  drop_viewer_from_dyn_cx(&mut viewer, &mut drop_cx);

  RenderResult {
    pixels: read.read_into_raw_unpadded_buffer(),
    format: read.info().format,
    stats,
  }
}

/// A trimmed and an untrimmed curved patch side by side.
fn synthetic_model() -> ParametricNormalizedModel {
  let patch = |offset: f64, trimmed: bool| {
    let mut control_points = Vec::new();
    for v in 0..4 {
      for u in 0..4 {
        let (x, y) = (u as f64 / 3. * 2. - 1., v as f64 / 3. * 2. - 1.);
        let z = 0.6 * (1. - x * x) * (1. - y * y);
        control_points.push(Vec4::new(x + offset, y, z, 1.));
      }
    }
    let boundary = |source| TrimLoop {
      curves: vec![TrimCurve {
        curve: RationalBezierCurve2::line(Vec2::new(0., 0.), Vec2::new(1., 0.)),
        source,
        exact: true,
      }],
    };
    TrimmedBezierPatch {
      surface: RationalBezierSurface::new(3, 3, control_points),
      loops: vec![boundary(if trimmed {
        TrimCurveSource::Edge {
          edge: EdgeId(0),
          reversed: false,
        }
      } else {
        TrimCurveSource::PatchBoundary
      })],
      flip_normal: false,
      face: FaceId(0),
      cell: (0, 0),
    }
  };
  ParametricNormalizedModel {
    patches: vec![patch(-1.05, false), patch(1.05, true)],
    instances: vec![FaceInstance {
      face: FaceId(0),
      patches: 0..2,
      transform: Mat4::identity(),
      flip_normal: false,
    }],
    faces: vec![FaceInfo { source_id: 0 }],
    edges: vec![EdgeInfo { source_id: 0 }],
    tolerance: 1e-6,
    length_unit_mm: None,
  }
}
