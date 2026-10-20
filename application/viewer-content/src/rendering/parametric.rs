//! Parametric model (`.pnm`) rendering, plan stage 2 in `extension/parametric-rendering/plan.md`.
//!
//! The model is drawn by an independent renderer, not as a scene model: no material, lighting,
//! picking or scene graph integration. Every frame a `TessellationProducer` gives the view
//! dependent tessellation (`TessellationFrameOutput`) for the first rendered viewport, which is
//! drawn in the main scene pass of each viewport with depth test. The output is in the camera
//! space of the view it was produced for, the draw maps it to the current view, so a producer
//! that lags behind the camera (the CPU reference runs on a worker thread) still draws at the
//! right place.

use parametric_normalized_model::{ParametricNormalizedModel, TrimmedBezierPatch};
use parametric_tessellation::{
  Camera as TessellationCamera, PARAM_BITS, TessellationConfig, Tessellator, camera_space_items,
};
use rendiation_parametric_rendering::tessellation::*;
pub use rendiation_parametric_rendering::tessellation::{
  TessellationColorMode, TessellationDrawSettings, TrimmedPatchStyle,
};

use crate::*;

/// State of the parametric model renderer, owned by `Viewer3dRenderingCtx`. One model at a time.
#[derive(Default)]
pub struct ViewerParametricRendering {
  model: Option<ParametricModelSource>,
  pub settings: ParametricRenderSettings,
  producer: Option<(TessellationProducerKind, Box<dyn TessellationProducer>)>,
}

impl ViewerParametricRendering {
  /// Replace the current model (if any).
  pub fn set_model(&mut self, source: ParametricModelSource) {
    log::info!(
      "parametric model {} loaded: {} patches ({} untrimmed), {} instances",
      source.name,
      source.model.patches.len(),
      source.untrimmed_patch_count,
      source.model.instances.len(),
    );
    self.model = Some(source);
    self.producer = None;
  }

  pub fn clear_model(&mut self) {
    self.model = None;
    self.producer = None;
  }

  pub fn model(&self) -> Option<&ParametricModelSource> {
    self.model.as_ref()
  }

  /// The model bounds in world space (control point hull bounds, scaled by the model scale).
  /// `None` without a model or for a model without any instance.
  pub fn world_bounds(&self) -> Option<Box3<f64>> {
    let bounds = self.model.as_ref()?.bounds;
    if bounds.is_empty() {
      return None;
    }
    let s = self.settings.model_scale;
    Some(Box3::new(bounds.min * s, bounds.max * s))
  }

  pub fn stats(&self) -> Option<&ParametricTessellationStats> {
    self.producer.as_ref()?.1.stats()
  }

  /// A tessellation is running asynchronously, the drawn output is older.
  pub fn is_busy(&self) -> bool {
    self.producer.as_ref().is_some_and(|(_, p)| p.is_busy())
  }

  /// Update the producer for the camera of the primary view and return what to draw this frame.
  pub fn prepare_frame(
    &mut self,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
    camera: &CameraTransform,
    viewport_size: Vec2<f32>,
    waker: &Waker,
  ) -> Option<ParametricFrame> {
    let source = self.model.as_ref()?;
    if !self.settings.enabled {
      return None;
    }
    let kind = self.settings.producer;
    if self.producer.as_ref().is_none_or(|(k, _)| *k != kind) {
      let producer = create_tessellation_producer(kind, gpu, &source.model)?;
      self.producer = Some((kind, producer));
    }
    let (_, producer) = self.producer.as_mut()?;

    let model_to_world = self.settings.model_to_world();
    let request = if self.settings.freeze {
      None
    } else {
      tessellation_camera(camera, viewport_size.into_f64(), model_to_world).map(|camera| {
        TessellationRequest {
          camera,
          config: self.settings.tessellation.config(),
          untrimmed_only: self.settings.untrimmed_only,
        }
      })
    };

    let (output, view) = producer.produce(gpu, encoder, request.as_ref(), waker)?;
    Some(ParametricFrame {
      output: output.clone(),
      view,
      model_to_world,
      draw: self.settings.draw,
    })
  }

  /// The settings ui, returns if anything changed.
  pub fn egui(&mut self, ui: &mut egui::Ui) -> bool {
    let before = self.settings;
    let s = &mut self.settings;
    ui.checkbox(&mut s.enabled, "draw parametric model");

    egui::ComboBox::from_label("tessellation producer")
      .selected_text(s.producer.name())
      .show_ui(ui, |ui| {
        for kind in [
          TessellationProducerKind::CpuReference,
          TessellationProducerKind::Gpu,
        ] {
          ui.selectable_value(&mut s.producer, kind, kind.name());
        }
      });

    ui.checkbox(&mut s.freeze, "freeze tessellation")
      .on_hover_text("keep drawing the last tessellation, to inspect it from other views");
    ui.checkbox(&mut s.untrimmed_only, "untrimmed patches only");

    let t = &mut s.tessellation;
    ui.add(
      egui::Slider::new(&mut t.pixel_error, 0.1..=16.0)
        .logarithmic(true)
        .text("pixel error"),
    );
    ui.add(egui::Slider::new(&mut t.edge_error_share, 0.05..=0.95).text("edge error share"));
    ui.add(
      egui::Slider::new(&mut t.max_triangle_size, 2.0..=512.0)
        .logarithmic(true)
        .text("max triangle size (px)"),
    );
    let mut k_max_log2 = t.k_max.trailing_zeros();
    ui.add(
      egui::Slider::new(&mut k_max_log2, 0..=(K_BUCKET_COUNT as u32 - 1))
        .custom_formatter(|v, _| format!("{}", 1u32 << v as u32))
        .text("k max"),
    );
    t.k_max = 1 << k_max_log2;
    ui.add(egui::Slider::new(&mut t.max_level, 0..=PARAM_BITS - k_max_log2).text("max level"));

    ui.separator();
    egui::ComboBox::from_label("color mode")
      .selected_text(format!("{:?}", s.draw.color_mode))
      .show_ui(ui, |ui| {
        for mode in [
          TessellationColorMode::Gray,
          TessellationColorMode::ByK,
          TessellationColorMode::ByLeaf,
          TessellationColorMode::ByPatch,
        ] {
          ui.selectable_value(&mut s.draw.color_mode, mode, format!("{mode:?}"));
        }
      });
    egui::ComboBox::from_label("trimmed patches")
      .selected_text(format!("{:?}", s.draw.trimmed_style))
      .show_ui(ui, |ui| {
        for style in [
          TrimmedPatchStyle::Normal,
          TrimmedPatchStyle::WashedOut,
          TrimmedPatchStyle::Hidden,
        ] {
          ui.selectable_value(&mut s.draw.trimmed_style, style, format!("{style:?}"));
        }
      })
      .response
      .on_hover_text("the tessellation ignores trimming, trimmed patches cover their whole cell");
    let mut color: [f32; 3] = s.draw.base_color.into();
    ui.horizontal(|ui| {
      ui.label("gray mode color");
      ui.color_edit_button_rgb(&mut color);
    });
    s.draw.base_color = color.into();
    ui.add(
      egui::DragValue::new(&mut s.model_scale)
        .speed(0.001)
        .range(1e-6..=1e6)
        .prefix("model scale: "),
    );

    ui.separator();
    if self.is_busy() {
      ui.label("tessellating...");
    }
    if let Some(stats) = self.stats() {
      stats.egui(ui);
    } else {
      ui.label("no tessellation result yet");
    }

    before != self.settings
  }
}

/// A loaded model with the information derived from it.
pub struct ParametricModelSource {
  pub model: Arc<ParametricNormalizedModel>,
  /// File name or other user facing name.
  pub name: String,
  /// Bounds of the control points of all instances, in model space.
  pub bounds: Box3<f64>,
  pub untrimmed_patch_count: usize,
}

impl ParametricModelSource {
  /// Can be called on a worker thread, it visits all control points.
  pub fn new(model: ParametricNormalizedModel, name: String) -> Self {
    let mut bounds = Box3::empty();
    for instance in &model.instances {
      for patch in &model.patches[instance.patches.clone()] {
        for p in &patch.surface.control_points {
          let p = Vec3::new(p.x, p.y, p.z) / p.w;
          bounds.expand_by_point(instance.transform * p);
        }
      }
    }
    Self {
      untrimmed_patch_count: model.patches.iter().filter(|p| p.is_untrimmed()).count(),
      model: Arc::new(model),
      name,
      bounds,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParametricRenderSettings {
  pub enabled: bool,
  pub producer: TessellationProducerKind,
  pub tessellation: TessellationParams,
  /// Only tessellate the patches without trim loops (the tessellation ignores trimming).
  pub untrimmed_only: bool,
  /// Stop updating the tessellation and keep drawing the last result.
  pub freeze: bool,
  /// Uniform scale from model units to world units.
  pub model_scale: f64,
  pub draw: TessellationDrawSettings,
}

impl Default for ParametricRenderSettings {
  fn default() -> Self {
    Self {
      enabled: true,
      producer: TessellationProducerKind::Gpu,
      tessellation: Default::default(),
      untrimmed_only: false,
      freeze: false,
      model_scale: 1.,
      draw: Default::default(),
    }
  }
}

impl ParametricRenderSettings {
  pub fn model_to_world(&self) -> Mat4<f64> {
    Mat4::scale(Vec3::splat(self.model_scale))
  }
}

/// `TessellationConfig` with equality, `k_max` is at most `1 << (K_BUCKET_COUNT - 1)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TessellationParams {
  pub pixel_error: f64,
  pub edge_error_share: f64,
  pub max_triangle_size: f64,
  pub k_max: u32,
  pub max_level: u32,
}

impl Default for TessellationParams {
  fn default() -> Self {
    let c = TessellationConfig::default();
    Self {
      pixel_error: c.pixel_error,
      edge_error_share: c.edge_error_share,
      max_triangle_size: c.max_triangle_size,
      k_max: c.k_max.min(1 << (K_BUCKET_COUNT - 1)),
      max_level: c.max_level,
    }
  }
}

impl TessellationParams {
  pub fn config(&self) -> TessellationConfig {
    let k_max = self
      .k_max
      .clamp(1, 1 << (K_BUCKET_COUNT - 1))
      .next_power_of_two();
    TessellationConfig {
      pixel_error: self.pixel_error,
      edge_error_share: self.edge_error_share,
      max_triangle_size: self.max_triangle_size,
      k_max,
      max_level: self.max_level.min(PARAM_BITS - k_max.trailing_zeros()),
    }
  }
}

/// What the parametric model draws in one frame, shared by all viewports.
pub struct ParametricFrame {
  pub output: TessellationFrameOutput,
  /// Model space to the camera space of the output.
  pub view: Mat4<f64>,
  pub model_to_world: Mat4<f64>,
  pub draw: TessellationDrawSettings,
}

impl ParametricFrame {
  /// The draw uniform for a view with `camera`.
  pub fn uniform(&self, camera: &CameraTransform) -> TessellationDrawUniform {
    let view = camera.view * self.model_to_world;
    let view_from_tessellation = view * self.view.inverse_or_identity();
    let clip_from_tessellation = camera.projection.into_f64() * view_from_tessellation;
    TessellationDrawUniform::new(
      clip_from_tessellation.into_f32(),
      view_from_tessellation.into_f32(),
      &self.draw,
      &self.output,
    )
  }
}

/// The input of a producer for one frame.
#[derive(Debug, Clone, Copy)]
pub struct TessellationRequest {
  /// `view` maps model space to the camera space (the viewer camera view times the model to
  /// world transform).
  pub camera: TessellationCamera,
  pub config: TessellationConfig,
  pub untrimmed_only: bool,
}

impl TessellationRequest {
  /// Bitwise equal camera and config.
  pub fn same_as(&self, other: &Self) -> bool {
    let (a, b) = (&self.camera, &other.camera);
    let (c, d) = (&self.config, &other.config);
    a.view == b.view
      && a.focal_length == b.focal_length
      && a.viewport == b.viewport
      && a.near == b.near
      && c.pixel_error == d.pixel_error
      && c.edge_error_share == d.edge_error_share
      && c.max_triangle_size == d.max_triangle_size
      && c.k_max == d.k_max
      && c.max_level == d.max_level
      && self.untrimmed_only == other.untrimmed_only
  }
}

/// A source of the per frame tessellation output, see `create_tessellation_producer` for the
/// implementations.
pub trait TessellationProducer {
  /// Called once per rendered frame with the current request, `None` if the tessellation should
  /// not follow the camera (frozen, or a camera the tessellation does not support). Returns the
  /// output to draw and the model to camera view matrix it was produced for, `None` if nothing is
  /// available yet. A producer that finishes asynchronously wakes `waker` to request a frame.
  fn produce(
    &mut self,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
    request: Option<&TessellationRequest>,
    waker: &Waker,
  ) -> Option<(&TessellationFrameOutput, Mat4<f64>)>;

  fn stats(&self) -> Option<&ParametricTessellationStats>;

  /// Work is running asynchronously, see `produce`.
  fn is_busy(&self) -> bool {
    false
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TessellationProducerKind {
  /// The `parametric-tessellation` crate on a worker thread, uploaded by
  /// `TessellationFrameOutput::from_cpu_reference`.
  CpuReference,
  /// The GPU tessellator of `rendiation-parametric-rendering` (plan stage 1).
  Gpu,
}

impl TessellationProducerKind {
  pub fn name(self) -> &'static str {
    match self {
      Self::CpuReference => "CPU reference",
      Self::Gpu => "GPU",
    }
  }
}

pub fn create_tessellation_producer(
  kind: TessellationProducerKind,
  gpu: &GPU,
  model: &Arc<ParametricNormalizedModel>,
) -> Option<Box<dyn TessellationProducer>> {
  match kind {
    TessellationProducerKind::CpuReference => Some(Box::new(
      CpuReferenceTessellationProducer::new(model.clone()),
    )),
    TessellationProducerKind::Gpu => Some(Box::new(GpuTessellationProducer::new(gpu, model))),
  }
}

/// The work is encoded into the frame encoder before the scene passes that draw the output, and
/// only when the request changed or the buffers grew after an overflow. The statistics read
/// wakes the waker when done, so an overflow is retried also when drawing on demand.
struct GpuTessellationProducer {
  tessellator: GpuTessellator,
  requested: Option<TessellationRequest>,
  /// The output is a set of reference counted buffer views, a clone is cheap.
  last: Option<(TessellationFrameOutput, Mat4<f64>)>,
  stats: Option<ParametricTessellationStats>,
}

impl GpuTessellationProducer {
  fn new(gpu: &GPU, model: &ParametricNormalizedModel) -> Self {
    Self {
      tessellator: GpuTessellator::new(gpu, model, TessellationConfig::default()),
      requested: None,
      last: None,
      stats: None,
    }
  }

  fn update_stats(&mut self) {
    let Some(s) = self.tessellator.last_stats() else {
      return;
    };
    let to_usize = |v: &[u32]| v.iter().map(|&x| x as usize).collect();
    self.stats = Some(ParametricTessellationStats {
      producer: TessellationProducerKind::Gpu.name(),
      items: s.items,
      rejected_items: s.rejected_patches,
      leaves: s.leaves as usize,
      triangles: s.triangles as usize,
      leaves_per_level: to_usize(&s.leaves_per_level),
      leaves_per_k: to_usize(&s.leaves_per_k),
      work_items_per_level: to_usize(&s.work_items),
      culled: s.culled as usize,
      forced_leaves: s.forced_leaves as usize,
      tessellation_ms: 0.,
      output_ms: 0.,
    });
  }
}

impl TessellationProducer for GpuTessellationProducer {
  fn produce(
    &mut self,
    gpu: &GPU,
    encoder: &mut GPUCommandEncoder,
    request: Option<&TessellationRequest>,
    waker: &Waker,
  ) -> Option<(&TessellationFrameOutput, Mat4<f64>)> {
    self.tessellator.poll_stats(gpu, waker);
    self.update_stats();
    // a frozen request keeps the buffers of the last encoded frame
    if let Some(request) = request
      && (self.tessellator.needs_encode() || self.requested.is_none_or(|r| !r.same_as(request)))
    {
      self.requested = Some(*request);
      self.tessellator.set_config(request.config);
      self.tessellator.set_untrimmed_only(request.untrimmed_only);
      let output = self.tessellator.encode_frame(gpu, encoder, &request.camera);
      self.last = Some((output.clone(), request.camera.view));
      // registers the waker on the statistics read started by this frame
      self.tessellator.poll_stats(gpu, waker);
    }
    self.last.as_ref().map(|(output, view)| (output, *view))
  }

  fn stats(&self) -> Option<&ParametricTessellationStats> {
    self.stats.as_ref()
  }
}

/// Tessellation statistics for the ui, common to all producers.
#[derive(Debug, Default, Clone)]
pub struct ParametricTessellationStats {
  pub producer: &'static str,
  /// Tessellation items, one per (instance, patch) occurrence passing the filter.
  pub items: usize,
  /// Items rejected because their degree is too high.
  pub rejected_items: usize,
  pub leaves: usize,
  pub triangles: usize,
  pub leaves_per_level: Vec<usize>,
  /// Indexed by `log2 k`.
  pub leaves_per_k: Vec<usize>,
  pub work_items_per_level: Vec<usize>,
  pub culled: usize,
  /// Leaves at the max level that did not meet the criteria.
  pub forced_leaves: usize,
  pub tessellation_ms: f64,
  /// Building and uploading the output.
  pub output_ms: f64,
}

impl ParametricTessellationStats {
  pub fn egui(&self, ui: &mut egui::Ui) {
    ui.label(format!("producer: {}", self.producer));
    ui.label(format!(
      "items: {} (rejected {}), culled work items: {}",
      self.items, self.rejected_items, self.culled
    ));
    ui.label(format!(
      "leaves: {}, triangles: {}, forced leaves: {}",
      self.leaves, self.triangles, self.forced_leaves
    ));
    ui.label(format!(
      "work items per level: {:?}",
      self.work_items_per_level
    ));
    ui.label(format!("leaves per level: {:?}", self.leaves_per_level));
    let per_k: Vec<_> = self
      .leaves_per_k
      .iter()
      .enumerate()
      .map(|(b, n)| format!("k{}: {n}", 1 << b))
      .collect();
    ui.label(format!("leaves per k: {}", per_k.join(", ")));
    ui.label(format!(
      "time: tessellation {:.1} ms, output {:.1} ms",
      self.tessellation_ms, self.output_ms
    ));
  }
}

/// Runs the CPU reference on a worker thread whenever the request changes and no job is running,
/// so it is throttled to one job at a time while the camera moves, the last finished output is
/// drawn meanwhile.
pub struct CpuReferenceTessellationProducer {
  model: Arc<ParametricNormalizedModel>,
  current: Option<(TessellationFrameOutput, Mat4<f64>)>,
  /// The request of the running or the last job.
  requested: Option<TessellationRequest>,
  job: Option<std::sync::mpsc::Receiver<CpuTessellationResult>>,
  stats: Option<ParametricTessellationStats>,
}

struct CpuTessellationResult {
  output: TessellationFrameOutput,
  view: Mat4<f64>,
  stats: ParametricTessellationStats,
}

impl CpuReferenceTessellationProducer {
  pub fn new(model: Arc<ParametricNormalizedModel>) -> Self {
    Self {
      model,
      current: None,
      requested: None,
      job: None,
      stats: None,
    }
  }

  fn poll_job(&mut self) {
    let Some(job) = &self.job else {
      return;
    };
    match job.try_recv() {
      Ok(result) => {
        self.current = Some((result.output, result.view));
        self.stats = Some(result.stats);
        self.job = None;
      }
      Err(std::sync::mpsc::TryRecvError::Empty) => {}
      Err(std::sync::mpsc::TryRecvError::Disconnected) => {
        // the request is kept, so the same failing request is not retried
        log::error!("cpu reference tessellation job failed");
        self.job = None;
      }
    }
  }
}

impl TessellationProducer for CpuReferenceTessellationProducer {
  fn produce(
    &mut self,
    gpu: &GPU,
    _encoder: &mut GPUCommandEncoder,
    request: Option<&TessellationRequest>,
    waker: &Waker,
  ) -> Option<(&TessellationFrameOutput, Mat4<f64>)> {
    self.poll_job();
    if let Some(request) = request
      && self.job.is_none()
      && self.requested.is_none_or(|r| !r.same_as(request))
    {
      self.requested = Some(*request);
      self.job = Some(spawn_cpu_tessellation(
        gpu.device.clone(),
        self.model.clone(),
        *request,
        waker.clone(),
      ));
    }
    self.current.as_ref().map(|(output, view)| (output, *view))
  }

  fn stats(&self) -> Option<&ParametricTessellationStats> {
    self.stats.as_ref()
  }

  fn is_busy(&self) -> bool {
    self.job.is_some()
  }
}

fn spawn_cpu_tessellation(
  device: GPUDevice,
  model: Arc<ParametricNormalizedModel>,
  request: TessellationRequest,
  waker: Waker,
) -> std::sync::mpsc::Receiver<CpuTessellationResult> {
  let (sender, receiver) = std::sync::mpsc::channel();
  let job = move || {
    let result = run_cpu_tessellation(&device, &model, &request);
    sender.send(result).ok();
    waker.wake();
  };

  #[cfg(not(target_family = "wasm"))]
  std::thread::Builder::new()
    .name("parametric cpu tessellation".into())
    .spawn(job)
    .unwrap();

  #[cfg(target_family = "wasm")]
  job();

  receiver
}

fn run_cpu_tessellation(
  device: &GPUDevice,
  model: &ParametricNormalizedModel,
  request: &TessellationRequest,
) -> CpuTessellationResult {
  let start = Instant::now();
  let untrimmed_only = request.untrimmed_only;
  let filter = |p: &TrimmedBezierPatch| !untrimmed_only || p.is_untrimmed();
  let (items, rejected_items) = camera_space_items(model, &request.camera, filter);
  let mut tessellator = Tessellator::new(&items, request.camera, request.config);
  let leaves = tessellator.run();
  let tessellated = Instant::now();
  let output =
    TessellationFrameOutput::from_cpu_reference(device, model, &mut tessellator, &leaves);
  let end = Instant::now();

  let s = &tessellator.stats;
  let stats = ParametricTessellationStats {
    producer: TessellationProducerKind::CpuReference.name(),
    items: items.len(),
    rejected_items,
    leaves: leaves.len(),
    triangles: s.triangles,
    leaves_per_level: s.leaves_per_level.clone(),
    leaves_per_k: s.leaves_per_k.clone(),
    work_items_per_level: s.work_items.clone(),
    culled: s.culled,
    forced_leaves: s.forced_leaves,
    tessellation_ms: tessellated.duration_since(start).as_secs_f64() * 1000.,
    output_ms: end.duration_since(tessellated).as_secs_f64() * 1000.,
  };
  log::info!(
    "parametric cpu tessellation: {} items, {} leaves, {} triangles, leaves per k {:?}, \
     tessellation {:.1} ms, output {:.1} ms",
    stats.items,
    stats.leaves,
    stats.triangles,
    stats.leaves_per_k,
    stats.tessellation_ms,
    stats.output_ms,
  );

  CpuTessellationResult {
    output,
    view: request.camera.view,
    stats,
  }
}

/// The pinhole camera of the tessellation for a perspective viewer camera, `None` for the other
/// projections. `model_to_world` is folded into the view, the tessellation runs in model space.
fn tessellation_camera(
  camera: &CameraTransform,
  viewport: Vec2<f64>,
  model_to_world: Mat4<f64>,
) -> Option<TessellationCamera> {
  let p = camera.projection.into_f64();
  // the clip w of a perspective projection is the negated view depth
  if p.c4 > -0.5 || viewport.y <= 0. {
    return None;
  }
  // the near plane is clip z = 0 for the standard depth and clip z = w for the reversed depth
  let near = p.d3 / if p.c3 < 0. { p.c3 } else { 1. + p.c3 };
  Some(TessellationCamera {
    view: camera.view * model_to_world,
    focal_length: viewport.y * 0.5 * p.b2,
    viewport,
    near,
  })
}

/// Draws the parametric model into the main scene pass of a view.
pub struct ParametricModelSceneContent<'a> {
  pub frame: &'a ParametricFrame,
  pub uniform: &'a UniformBufferDataView<TessellationDrawUniform>,
  pub camera: &'a CameraGPU,
  pub reversed_depth: bool,
}

impl PassContent for ParametricModelSceneContent<'_> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    let base = default_dispatcher(pass, self.reversed_depth);
    render_tessellation_output(
      &mut pass.ctx,
      &self.frame.output,
      self.uniform,
      &[&base, self.camera],
      &[&ApplyCameraJitter],
    );
  }
}

/// Adds the TAA jitter of the camera to the `ClipPosition` registered by the inner component, the
/// same way as `compute_jittered_clip_space_position`.
struct ApplyCameraJitter;

impl ShaderHashProvider for ApplyCameraJitter {
  shader_hash_type_id! {}
}

impl ShaderPassBuilder for ApplyCameraJitter {}

impl GraphicsShaderProvider for ApplyCameraJitter {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, _| {
      let clip = builder.query::<ClipPosition>();
      let texel_size = builder.query::<TexelSize>();
      let jitter = texel_size * builder.query::<CameraJitter>() * clip.w();
      let jitter: Node<Vec4<f32>> = (jitter, val(0.), val(0.)).into();
      builder.register::<ClipPosition>(clip + jitter);
    })
  }
}
