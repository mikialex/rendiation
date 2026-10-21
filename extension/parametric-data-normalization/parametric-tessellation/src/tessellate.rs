use fast_hash_collection::FastHashMap;
use parametric_normalized_model::*;
use rendiation_algebra::*;

use crate::trim::*;
use crate::*;

#[derive(Debug, Clone, Copy)]
pub struct TessellationConfig {
  /// Max screen space deviation of the triangles from the surface, in pixels.
  pub pixel_error: f64,
  /// Part of `pixel_error` given to the edge linearization, the rest goes to the leaf grid.
  /// The bound of a triangle is the grid part plus the edge part per nested edge interpolation
  /// of its vertices (`TessellationStats::position_chains`), the measured error stays well
  /// below `pixel_error` with an even share, while a smaller share makes edges need more
  /// break points than the grid.
  pub edge_error_share: f64,
  /// Max projected size of a leaf grid cell, in pixels.
  pub max_triangle_size: f64,
  /// Max leaf grid resolution and max pieces of an edge linearization, a power of two.
  pub k_max: u32,
  pub max_level: u32,
}

impl Default for TessellationConfig {
  fn default() -> Self {
    Self {
      pixel_error: 0.5,
      edge_error_share: 0.5,
      max_triangle_size: 32.,
      k_max: 16,
      max_level: 10,
    }
  }
}

impl TessellationConfig {
  fn interior_tolerance(&self) -> f64 {
    self.pixel_error * (1. - self.edge_error_share)
  }

  fn edge_tolerance(&self) -> f64 {
    self.pixel_error * self.edge_error_share
  }
}

/// Pinhole camera, looking along -z of its own space.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
  /// World to camera.
  pub view: Mat4<f64>,
  /// Pixels per length unit at depth 1.
  pub focal_length: f64,
  /// Viewport size in pixels.
  pub viewport: Vec2<f64>,
  pub near: f64,
}

impl Camera {
  pub fn look_at(
    eye: Vec3<f64>,
    target: Vec3<f64>,
    up: Vec3<f64>,
    fov_y: f64,
    viewport: Vec2<f64>,
    near: f64,
  ) -> Self {
    Self {
      view: Mat4::lookat(eye, target, up).inverse_or_identity(),
      focal_length: viewport.y * 0.5 / (fov_y * 0.5).tan(),
      viewport,
      near,
    }
  }

  pub fn camera_to_world(&self) -> Mat4<f64> {
    self.view.inverse_or_identity()
  }

  pub fn fov_y(&self) -> f64 {
    2. * (self.viewport.y * 0.5 / self.focal_length).atan()
  }

  /// Pixels per length unit at camera space position `p`.
  pub fn pixel_scale(&self, p: Vec3<f64>) -> f64 {
    self.focal_length / -p.z
  }

  /// The camera space box is completely outside the view frustum (no far plane).
  pub fn culls(&self, min: Vec3<f64>, max: Vec3<f64>) -> bool {
    let far_depth = -min.z;
    let tx = self.viewport.x * 0.5 / self.focal_length;
    let ty = self.viewport.y * 0.5 / self.focal_length;
    far_depth < self.near
      || min.x > tx * far_depth
      || max.x < -tx * far_depth
      || min.y > ty * far_depth
      || max.y < -ty * far_depth
  }

  /// The smallest margin of the `culls` comparisons in their short circuit order: the near
  /// plane relative to `near`, the frustum sides in pixels relative to `pixel_reference`.
  pub fn cull_margin(&self, min: Vec3<f64>, max: Vec3<f64>, pixel_reference: f64) -> f64 {
    let far_depth = -min.z;
    let mut margin = ((far_depth - self.near) / self.near).abs();
    if far_depth < self.near {
      return margin;
    }
    let tx = self.viewport.x * 0.5 / self.focal_length;
    let ty = self.viewport.y * 0.5 / self.focal_length;
    let scale = self.focal_length / far_depth / pixel_reference;
    let sides = [
      (min.x, tx * far_depth),
      (-max.x, tx * far_depth),
      (min.y, ty * far_depth),
      (-max.y, ty * far_depth),
    ];
    for (value, limit) in sides {
      margin = margin.min(((value - limit) * scale).abs());
      if value > limit {
        break;
      }
    }
    margin
  }
}

/// One (instance, patch) occurrence to tessellate.
#[derive(Debug, Clone)]
pub struct TessellationItem {
  pub instance: u32,
  pub patch: u32,
  /// The patch in camera space. Patches of the same instance sharing boundary control points
  /// keep them bitwise equal, which the cross patch consistency relies on.
  pub surface: RationalBezierSurface,
}

/// Same limit as the GPU evaluation (`MAX_GPU_DEGREE` in `rendiation-parametric-rendering`).
pub const MAX_DEGREE: usize = 14;

/// All instance patches accepted by `filter` in camera space, plus the number of patches
/// rejected for exceeding `MAX_DEGREE`. The control points are taken relative to their face
/// origin and mapped by `item_to_camera`, the same data and matrix the GPU version uses in f32.
pub fn camera_space_items(
  model: &ParametricNormalizedModel,
  camera: &Camera,
  filter: impl Fn(&TrimmedBezierPatch) -> bool,
) -> (Vec<TessellationItem>, usize) {
  let origins = face_origins(model);
  let mut items = Vec::new();
  let mut rejected = 0;
  for (instance_index, instance) in model.instances.iter().enumerate() {
    let origin = origins[instance.face.index()];
    let to_camera = item_to_camera(camera, instance, origin);
    for patch_index in instance.patches.clone() {
      let patch = &model.patches[patch_index];
      if !filter(patch) {
        continue;
      }
      if patch.surface.degree_u > MAX_DEGREE || patch.surface.degree_v > MAX_DEGREE {
        rejected += 1;
        continue;
      }
      items.push(TessellationItem {
        instance: instance_index as u32,
        patch: patch_index as u32,
        surface: relative_to_origin(&patch.surface, origin).transform(to_camera),
      });
    }
  }
  (items, rejected)
}

/// Per `FaceId`, the center of the Euclidean control point box of all patches of the face
/// (zero for faces without patches).
///
/// Control points are stored relative to this origin, so the f32 values stay small, and all
/// patches of a face share it: neighbor patches with bitwise equal boundary control points keep
/// them bitwise equal after subtracting the origin and applying the same matrix. A per patch
/// origin would break that.
pub fn face_origins(model: &ParametricNormalizedModel) -> Vec<Vec3<f64>> {
  let inf = f64::INFINITY;
  let mut boxes = vec![(Vec3::new(inf, inf, inf), Vec3::new(-inf, -inf, -inf)); model.faces.len()];
  for patch in &model.patches {
    let (min, max) = &mut boxes[patch.face.index()];
    for p in &patch.surface.control_points {
      let p = p.to_euclid();
      *min = min.min(p);
      *max = max.max(p);
    }
  }
  boxes
    .into_iter()
    .map(|(min, max)| {
      if min.x <= max.x {
        (min + max) * 0.5
      } else {
        Vec3::new(0., 0., 0.)
      }
    })
    .collect()
}

/// The surface with its control points translated by `-origin` (homogeneous `p·w - o·w`).
pub fn relative_to_origin(
  surface: &RationalBezierSurface,
  origin: Vec3<f64>,
) -> RationalBezierSurface {
  let control_points = surface
    .control_points
    .iter()
    .map(|p| {
      Vec4::new(
        p.x - origin.x * p.w,
        p.y - origin.y * p.w,
        p.z - origin.z * p.w,
        p.w,
      )
    })
    .collect();
  RationalBezierSurface::new(surface.degree_u, surface.degree_v, control_points)
}

/// Face origin relative control points to camera space: `view · instance.transform ·
/// translate(origin)`, composed in f64.
pub fn item_to_camera(camera: &Camera, instance: &FaceInstance, origin: Vec3<f64>) -> Mat4<f64> {
  camera.view * instance.transform * Mat4::translate(origin)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Leaf {
  pub key: SubPatchKey,
  /// Grid resolution, the leaf is drawn as `k×k` cells of two triangles.
  pub k: u32,
  /// Partially covered by the trim loops of its patch, so drawn with per fragment trimming.
  /// Always false without trim classification (`Tessellator::with_trims`).
  pub trimmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitReason {
  /// The control box reaches the near plane.
  NearPlane,
  /// The grid error needs more than `k_max`.
  Interior,
  /// The projected size needs more than `k_max` to respect `max_triangle_size`.
  Size,
  /// An edge has no linearization with at most `k_max` pieces.
  Edge,
}

#[derive(Debug, Default, Clone)]
pub struct TessellationStats {
  /// Work items processed per level.
  pub work_items: Vec<usize>,
  pub leaves_per_level: Vec<usize>,
  /// Leaf count by `log2 k`.
  pub leaves_per_k: Vec<usize>,
  pub culled: usize,
  /// Work items outside the trim loops of their patch (with trim classification).
  pub trim_culled: usize,
  /// Leaves inside the trim loops of a patch that has trim data (no fragment trimming).
  pub trim_inside_leaves: usize,
  /// Leaves partially covered by the trim loops (`Leaf::trimmed`).
  pub trim_partial_leaves: usize,
  /// Indexed by `SplitReason as usize`.
  pub splits: [usize; 4],
  /// Leaves at `max_level` that did not meet the criteria, output with `k_max`.
  pub forced_leaves: usize,
  /// Leaves whose `k` was raised above the grid criteria to contain the edge break points.
  pub edge_raised_leaves: usize,
  /// Triangle count if no `k` had been raised for the edges (not watertight).
  pub triangles_without_edge_raise: usize,
  /// Edge linearization tests (excluding cache hits).
  pub edge_tests: usize,
  /// Edge segments at `max_level` accepted without meeting the criterion.
  pub forced_edge_segments: usize,
  /// Leaf boundary vertices by the number of nested edge interpolations their position needed.
  pub position_chains: Vec<usize>,
  pub triangles: usize,
}

/// The CPU reference of the view dependent adaptive tessellation: every (instance, patch) is
/// split level by level into dyadic sub patches until the leaf grid error, its size and its
/// edges meet the screen space criteria. See `crate` docs for the edge consistency scheme.
pub struct Tessellator<'a> {
  items: &'a [TessellationItem],
  camera: Camera,
  config: TessellationConfig,
  trims: Option<&'a TrimEncoding>,
  trim_dilation: f32,
  states: Vec<PatchState>,
  /// The smallest decision margin while `decision_margin` runs.
  margin: Option<f64>,
  pub stats: TessellationStats,
}

impl<'a> Tessellator<'a> {
  pub fn new(items: &'a [TessellationItem], camera: Camera, config: TessellationConfig) -> Self {
    assert!(config.k_max.is_power_of_two());
    assert!(config.max_level + config.k_max.trailing_zeros() <= PARAM_BITS);
    Self {
      items,
      camera,
      config,
      trims: None,
      trim_dilation: 0.,
      states: items.iter().map(|_| PatchState::default()).collect(),
      margin: None,
      stats: TessellationStats::default(),
    }
  }

  /// Classify every work item against the trim loops of its patch (`trims` is indexed by
  /// `TessellationItem::patch`) after the frustum cull: the items outside are culled, the leaves
  /// partially covered get `Leaf::trimmed`.
  pub fn with_trims(mut self, trims: &'a TrimEncoding) -> Self {
    self.trims = Some(trims);
    self
  }

  /// The fragments dilate the trim by `pixels` (`dilation_margin`), the classification keeps
  /// the sub patches within that distance.
  pub fn with_trim_dilation(mut self, pixels: f32) -> Self {
    self.trim_dilation = pixels;
    self
  }

  pub fn classifies_trims(&self) -> bool {
    self.trims.is_some()
  }

  pub fn items(&self) -> &'a [TessellationItem] {
    self.items
  }

  pub fn camera(&self) -> &Camera {
    &self.camera
  }

  pub fn config(&self) -> &TessellationConfig {
    &self.config
  }

  pub fn run(&mut self) -> Vec<Leaf> {
    let mut queue: Vec<_> = (0..self.items.len() as u32)
      .map(SubPatchKey::root)
      .collect();
    let mut leaves = Vec::new();
    let k_levels = self.config.k_max.trailing_zeros() as usize + 1;
    self.stats.leaves_per_k.resize(k_levels, 0);
    for _ in 0..=self.config.max_level {
      if queue.is_empty() {
        break;
      }
      self.stats.work_items.push(queue.len());
      let mut level_leaves = 0;
      let mut next = Vec::new();
      for key in queue {
        match self.patch(key.item).classify(key) {
          Decision::Cull => self.stats.culled += 1,
          Decision::TrimCull => self.stats.trim_culled += 1,
          Decision::Split(reason) => {
            self.stats.splits[reason as usize] += 1;
            next.extend(key.children());
          }
          Decision::Leaf {
            k,
            k_grid,
            forced,
            trim,
          } => {
            let trimmed = trim == Some(RectClass::Partial);
            self.stats.trim_partial_leaves += trimmed as usize;
            self.stats.trim_inside_leaves += (trim == Some(RectClass::Inside)) as usize;
            self.stats.forced_leaves += forced as usize;
            self.stats.edge_raised_leaves += (k > k_grid) as usize;
            self.stats.triangles_without_edge_raise += 2 * (k_grid * k_grid) as usize;
            self.stats.leaves_per_k[k.trailing_zeros() as usize] += 1;
            self.stats.triangles += 2 * (k * k) as usize;
            level_leaves += 1;
            leaves.push(Leaf { key, k, trimmed });
          }
        }
      }
      self.stats.leaves_per_level.push(level_leaves);
      queue = next;
    }
    assert!(queue.is_empty());
    leaves
  }

  /// The leaf grid in camera space. Boundary vertices are placed on the edge linearization
  /// shared with the neighbors, interior vertices are evaluated on the surface.
  pub fn leaf_mesh(&mut self, leaf: &Leaf) -> LeafMesh {
    let mut patch = self.patch(leaf.key.item);
    let k = leaf.k;
    LeafMesh::grid(leaf, |i, j, u, v| {
      if i == 0 || j == 0 || i == k || j == k {
        let p = patch.position(u, v);
        let chains = &mut patch.stats.position_chains;
        if chains.len() <= p.chain as usize {
          chains.resize(p.chain as usize + 1, 0);
        }
        chains[p.chain as usize] += 1;
        p.position
      } else {
        patch.surface.eval(param_to_f64(u), param_to_f64(v))
      }
    })
  }

  /// The smallest relative margin `|value - threshold| / |threshold|` of the threshold
  /// decisions taken when classifying `key`: cull (frustum sides in pixels relative to
  /// `pixel_error`), near plane, grid error and size per tested `k`, and every edge
  /// linearization test of the flat ancestor search. A tessellation computed with different
  /// rounding (the f32 GPU version) can only decide differently on keys with a small margin.
  pub fn decision_margin(&mut self, key: SubPatchKey) -> f64 {
    self.margin = Some(f64::INFINITY);
    self.patch(key.item).classify(key);
    self.margin.take().unwrap()
  }

  /// The leaf meshes computed like the GPU vertex passes instead of the cached recursion of
  /// `leaf_mesh`, with the same result bits. Round 0 requests the boundary vertices of all
  /// leaves. A request inside a linearization piece records the interpolation and requests the
  /// two piece ends in the next round, any other request is evaluated exactly. The rounds are
  /// then resolved backwards by interpolation. Also returns the request count per round.
  pub fn leaf_meshes_by_rounds(&mut self, leaves: &[Leaf]) -> (Vec<LeafMesh>, Vec<usize>) {
    let mut meshes = Vec::with_capacity(leaves.len());
    let mut requests = Vec::new();
    let mut targets = Vec::new();
    for (index, leaf) in leaves.iter().enumerate() {
      let surface = &self.items[leaf.key.item as usize].surface;
      let mesh = LeafMesh::grid(leaf, |i, j, u, v| {
        if i == 0 || j == 0 || i == leaf.k || j == leaf.k {
          requests.push((leaf.key.item, u, v));
          targets.push((index, (j * (leaf.k + 1) + i) as usize));
          Vec3::new(0., 0., 0.)
        } else {
          surface.eval(param_to_f64(u), param_to_f64(v))
        }
      });
      meshes.push(mesh);
    }

    let mut records = Vec::new();
    while !requests.is_empty() {
      let mut next = Vec::new();
      let record: Vec<_> = requests
        .iter()
        .map(|&(item, u, v)| {
          let mut patch = self.patch(item);
          match patch.piece(u, v) {
            Some(piece) => {
              next.push((item, piece.a.0, piece.a.1));
              next.push((item, piece.b.0, piece.b.1));
              Err((piece.t, next.len() - 2, next.len() - 1))
            }
            None => Ok(patch.exact(u, v)),
          }
        })
        .collect();
      records.push(record);
      requests = next;
    }

    let mut values: Vec<Vec3<f64>> = Vec::new();
    for record in records.iter().rev() {
      values = record
        .iter()
        .map(|r| match *r {
          Ok(p) => p,
          Err((t, a, b)) => blend(values[a], values[b], t),
        })
        .collect();
    }
    for ((mesh, vertex), p) in targets.into_iter().zip(values) {
      meshes[mesh].positions[vertex] = p;
    }
    (meshes, records.iter().map(Vec::len).collect())
  }

  /// Edge linearization piece count of the canonical `segment` of `line`, see
  /// `linearization` (exposed for comparison with the GPU version, like the next two).
  pub fn linearization_of(&mut self, item: u32, line: LineKey, segment: Segment) -> Option<u32> {
    self.patch(item).linearization(line, segment)
  }

  /// The coarsest linearizable segment containing the canonical `segment` and its piece count.
  pub fn flat_ancestor_of(
    &mut self,
    item: u32,
    line: LineKey,
    segment: Segment,
  ) -> Option<(Segment, u32)> {
    self.patch(item).flat_ancestor(line, segment)
  }

  /// The linearization piece of `line` containing canonical parameter `p` in its interior.
  pub fn locate_on(&mut self, item: u32, line: LineKey, p: u32) -> Option<Segment> {
    self.patch(item).locate(line, p)
  }

  /// The iso line curve in its canonical direction, see `LineCurve`.
  pub fn line_curve(&mut self, item: u32, line: LineKey) -> &LineCurve {
    self.patch(item).line(line);
    &self.states[item as usize].lines[&line]
  }

  fn patch(&mut self, item: u32) -> PatchEval<'_> {
    let patch = self.items[item as usize].patch as usize;
    PatchEval {
      surface: &self.items[item as usize].surface,
      trim: self
        .trims
        .filter(|trims| trims.header(patch).kind != TRIM_KIND_INSIDE)
        .map(|trims| (trims, patch)),
      trim_dilation: self.trim_dilation,
      camera: &self.camera,
      config: &self.config,
      state: &mut self.states[item as usize],
      margin: &mut self.margin,
      stats: &mut self.stats,
    }
  }
}

pub struct LeafMesh {
  pub k: u32,
  /// `(k + 1)²` grid vertices, u index fastest.
  pub params: Vec<(u32, u32)>,
  /// Camera space.
  pub positions: Vec<Vec3<f64>>,
}

impl LeafMesh {
  /// The `(k + 1)²` grid of `leaf` with the positions given by `position(i, j, u, v)`.
  pub fn grid(leaf: &Leaf, mut position: impl FnMut(u32, u32, u32, u32) -> Vec3<f64>) -> Self {
    let (us, vs) = (leaf.key.u_segment(), leaf.key.v_segment());
    let k = leaf.k;
    let (du, dv) = (us.length() / k, vs.length() / k);
    let mut params = Vec::with_capacity(((k + 1) * (k + 1)) as usize);
    let mut positions = Vec::with_capacity(params.capacity());
    for j in 0..=k {
      for i in 0..=k {
        let (u, v) = (us.start() + i * du, vs.start() + j * dv);
        params.push((u, v));
        positions.push(position(i, j, u, v));
      }
    }
    Self {
      k,
      params,
      positions,
    }
  }

  /// Two triangles per grid cell, counter clockwise in the parameter domain (so facing
  /// `∂S/∂u × ∂S/∂v`).
  pub fn triangles(&self) -> impl Iterator<Item = [u32; 3]> + '_ {
    let row = self.k + 1;
    (0..self.k).flat_map(move |j| {
      (0..self.k).flat_map(move |i| {
        let v00 = j * row + i;
        let (v10, v01, v11) = (v00 + 1, v00 + row, v00 + row + 1);
        [[v00, v10, v11], [v00, v11, v01]]
      })
    })
  }
}

/// An iso line of a patch as a curve in camera space. Patch boundary lines are stored in a
/// canonical direction (lexicographically smaller control point sequence first), so two
/// patches sharing the boundary control points derive the same curve and parameters whatever
/// their own orientation. Interior lines keep the patch direction.
#[derive(Debug, Clone)]
pub struct LineCurve {
  pub curve: RationalBezierCurve3,
  /// The canonical direction is the reverse of the patch parameter direction.
  pub reversed: bool,
}

impl LineCurve {
  fn new(surface: &RationalBezierSurface, line: LineKey) -> Self {
    if !line.is_patch_boundary() {
      return Self {
        curve: surface.iso_curve(line.axis, param_to_f64(line.coord)),
        reversed: false,
      };
    }
    let (n, m) = (surface.degree_u, surface.degree_v);
    let last = line.coord == PARAM_ONE;
    let mut points: Vec<_> = match line.axis {
      Axis::U => (0..=m)
        .map(|j| surface.control_point(if last { n } else { 0 }, j))
        .collect(),
      Axis::V => (0..=n)
        .map(|i| surface.control_point(i, if last { m } else { 0 }))
        .collect(),
    };
    let order = |a: &Vec4<f64>, b: &Vec4<f64>| {
      [(a.x, b.x), (a.y, b.y), (a.z, b.z), (a.w, b.w)]
        .into_iter()
        .map(|(x, y)| x.total_cmp(&y))
        .find(|o| o.is_ne())
        .unwrap_or(std::cmp::Ordering::Equal)
    };
    let reversed = points
      .iter()
      .rev()
      .zip(points.iter())
      .map(|(a, b)| order(a, b))
      .find(|o| o.is_ne())
      .is_some_and(|o| o.is_lt());
    if reversed {
      points.reverse();
    }
    Self {
      curve: RationalBezierCurve::new(points),
      reversed,
    }
  }

  /// Patch parameter along the line to canonical parameter, also the inverse.
  pub fn canonical(&self, p: u32) -> u32 {
    if self.reversed { PARAM_ONE - p } else { p }
  }

  pub fn canonical_segment(&self, s: Segment) -> Segment {
    if self.reversed { s.reversed() } else { s }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct VertexPosition {
  pub position: Vec3<f64>,
  /// Number of nested edge interpolations.
  pub chain: u32,
}

#[derive(Default)]
struct PatchState {
  lines: FastHashMap<LineKey, LineCurve>,
  /// Edge linearization piece count by canonical segment, `None` if not linearizable.
  pieces: FastHashMap<(LineKey, Segment), Option<u32>>,
  positions: FastHashMap<(u32, u32), VertexPosition>,
}

enum Decision {
  Cull,
  /// Outside the trim loops.
  TrimCull,
  Split(SplitReason),
  /// `trim` is the trim classification of the leaf, `None` for a patch without trim data or
  /// without trim classification.
  Leaf {
    k: u32,
    k_grid: u32,
    forced: bool,
    trim: Option<RectClass>,
  },
}

struct PatchEval<'a> {
  surface: &'a RationalBezierSurface,
  /// The trims and the patch index, only for a patch with trim data (not `TRIM_KIND_INSIDE`).
  trim: Option<(&'a TrimEncoding, usize)>,
  trim_dilation: f32,
  camera: &'a Camera,
  config: &'a TessellationConfig,
  state: &'a mut PatchState,
  margin: &'a mut Option<f64>,
  stats: &'a mut TessellationStats,
}

/// A point inside a linearization piece: `blend(position(a), position(b), t)`, with `a`, `b`
/// the piece ends in canonical line order.
struct Piece {
  t: f64,
  a: (u32, u32),
  b: (u32, u32),
}

impl PatchEval<'_> {
  fn classify(&mut self, key: SubPatchKey) -> Decision {
    let (us, vs) = (key.u_segment(), key.v_segment());
    let sub = self.surface.sub_patch(
      (param_to_f64(us.start()), param_to_f64(us.end())),
      (param_to_f64(vs.start()), param_to_f64(vs.end())),
    );
    let bounds = surface_bounds(&sub);
    if let Some(margin) = self.margin.as_mut() {
      let m = self
        .camera
        .cull_margin(bounds.min, bounds.max, self.config.pixel_error);
      *margin = margin.min(m);
    }
    if self.camera.culls(bounds.min, bounds.max) {
      return Decision::Cull;
    }
    let trim = self.trim.map(|(trims, patch)| {
      let rect = TrimRect {
        min: Vec2::new(param_to_f64(us.start()), param_to_f64(vs.start())).into_f32(),
        max: Vec2::new(param_to_f64(us.end()), param_to_f64(vs.end())).into_f32(),
      };
      let depth = -bounds.max.z;
      let margin = if depth > self.camera.near {
        let size = (bounds.max - bounds.min).length() * self.camera.focal_length / depth;
        dilation_margin(self.trim_dilation, key.level, size as f32)
      } else {
        dilation_margin(self.trim_dilation, key.level, 0.)
      };
      trims.classify_rect(patch, &rect.expanded(margin))
    });
    if trim == Some(RectClass::Outside) {
      return Decision::TrimCull;
    }
    let result = self.grid_resolution(&bounds).and_then(|k_grid| {
      let k = key
        .edges()
        .into_iter()
        .try_fold(k_grid, |k, (line, segment)| {
          let segment = self.line(line).canonical_segment(segment);
          let (flat, pieces) = self.flat_ancestor(line, segment).ok_or(SplitReason::Edge)?;
          // The leaf edge must contain every break point of the linearization inside it.
          Ok(k.max(pieces >> (segment.level - flat.level)))
        })?;
      Ok((k, k_grid))
    });
    match result {
      Ok((k, k_grid)) => Decision::Leaf {
        k,
        k_grid,
        forced: false,
        trim,
      },
      Err(reason) if key.level < self.config.max_level => Decision::Split(reason),
      Err(_) => Decision::Leaf {
        k: self.config.k_max,
        k_grid: self.config.k_max,
        forced: true,
        trim,
      },
    }
  }

  /// The smallest power of two grid resolution meeting the interior error and size criteria.
  fn grid_resolution(&mut self, bounds: &SurfaceBounds) -> Result<u32, SplitReason> {
    let depth = -bounds.max.z;
    if self.at_most(depth, self.camera.near) {
      return Err(SplitReason::NearPlane);
    }
    let scale = self.camera.focal_length / depth;
    let tolerance = self.config.interior_tolerance() / scale;
    let k_error = self
      .powers_of_two()
      .find(|&k| self.at_most(bounds.grid_error(k), tolerance))
      .ok_or(SplitReason::Interior)?;
    let size = (bounds.max - bounds.min).length() * scale;
    let max_size = self.config.max_triangle_size;
    let k_size = self
      .powers_of_two()
      .find(|&k| self.at_most(size / k as f64, max_size))
      .ok_or(SplitReason::Size)?;
    Ok(k_error.max(k_size))
  }

  /// `value <= threshold`, recording the decision margin when requested.
  fn at_most(&mut self, value: f64, threshold: f64) -> bool {
    if let Some(margin) = self.margin.as_mut() {
      *margin = margin.min(((value - threshold) / threshold).abs());
    }
    value <= threshold
  }

  fn powers_of_two(&self) -> impl Iterator<Item = u32> + use<> {
    (0..=self.config.k_max.trailing_zeros()).map(|e| 1 << e)
  }

  fn line(&mut self, line: LineKey) -> &LineCurve {
    self
      .state
      .lines
      .entry(line)
      .or_insert_with(|| LineCurve::new(self.surface, line))
  }

  /// Number of uniform pieces linearizing the canonical `segment` of `line` within the edge
  /// tolerance, `None` if more than `k_max` are needed. Segments at `max_level` always succeed.
  /// Must be a pure function of the line curve and the segment: both sides of an edge rely on
  /// getting the same answer.
  fn linearization(&mut self, line: LineKey, segment: Segment) -> Option<u32> {
    let recording = self.margin.is_some();
    if let Some(&cached) = self.state.pieces.get(&(line, segment))
      && !recording
    {
      return cached;
    }
    let bounds = {
      let curve = &self.line(line).curve;
      curve_bounds(&curve.sub_curve(param_to_f64(segment.start()), param_to_f64(segment.end())))
    };
    self.stats.edge_tests += !recording as usize;
    let depth = -bounds.max.z;
    let mut pieces = None;
    if !self.at_most(depth, self.camera.near) {
      let tolerance = self.config.edge_tolerance() * depth / self.camera.focal_length;
      pieces = self
        .powers_of_two()
        .find(|&m| self.at_most(bounds.polyline_error(m), tolerance));
    }
    if pieces.is_none() && segment.level >= self.config.max_level {
      self.stats.forced_edge_segments += !recording as usize;
      pieces = Some(self.config.k_max);
    }
    self.state.pieces.insert((line, segment), pieces);
    pieces
  }

  /// The coarsest linearizable segment containing the canonical `segment`, searched from the
  /// line's root cell down, with its piece count.
  fn flat_ancestor(&mut self, line: LineKey, segment: Segment) -> Option<(Segment, u32)> {
    (line.level()..=segment.level).find_map(|level| {
      let s = segment.ancestor(level);
      self.linearization(line, s).map(|m| (s, m))
    })
  }

  /// The linearization piece containing canonical parameter `p` in its interior, `None` if
  /// `p` is a break point (or no linearized segment contains it in its interior).
  fn locate(&mut self, line: LineKey, p: u32) -> Option<Segment> {
    let mut segment = Segment::containing(line.level(), p)?;
    loop {
      if let Some(m) = self.linearization(line, segment) {
        return Segment::containing(segment.level + m.trailing_zeros(), p);
      }
      segment = Segment::containing(segment.level + 1, p)?;
    }
  }

  /// Position of a sub patch edge vertex. If the point lies inside a piece of the
  /// linearization of the iso line through it, it is interpolated between the piece ends
  /// (themselves positioned recursively), otherwise evaluated exactly. A point can lie inside a
  /// piece of at most one of its two iso lines, since a piece of the line `u = c` is not
  /// longer than the cell of the level of `c`.
  fn position(&mut self, u: u32, v: u32) -> VertexPosition {
    if let Some(&cached) = self.state.positions.get(&(u, v)) {
      return cached;
    }
    let result = match self.piece(u, v) {
      Some(piece) => {
        let pa = self.position(piece.a.0, piece.a.1);
        let pb = self.position(piece.b.0, piece.b.1);
        VertexPosition {
          position: blend(pa.position, pb.position, piece.t),
          chain: pa.chain.max(pb.chain) + 1,
        }
      }
      None => VertexPosition {
        position: self.exact(u, v),
        chain: 0,
      },
    };
    self.state.positions.insert((u, v), result);
    result
  }

  /// The linearization piece containing `(u, v)` in its interior, on the line `u = const`
  /// first, then on `v = const`.
  fn piece(&mut self, u: u32, v: u32) -> Option<Piece> {
    self
      .piece_on_line(LineKey::new(Axis::U, u), v)
      .or_else(|| self.piece_on_line(LineKey::new(Axis::V, v), u))
  }

  fn piece_on_line(&mut self, line: LineKey, along: u32) -> Option<Piece> {
    let curve = self.line(line);
    let p = curve.canonical(along);
    let piece = self.locate(line, p)?;
    let curve = self.line(line);
    let (a, b) = (piece.start(), piece.end());
    Some(Piece {
      t: (p - a) as f64 / (b - a) as f64,
      a: line.point(curve.canonical(a)),
      b: line.point(curve.canonical(b)),
    })
  }

  /// Patch boundary points are evaluated on the canonical boundary curve so both patches
  /// sharing it get the same bits.
  fn exact(&mut self, u: u32, v: u32) -> Vec3<f64> {
    let boundary = if u == 0 || u == PARAM_ONE {
      Some((LineKey::new(Axis::U, u), v))
    } else if v == 0 || v == PARAM_ONE {
      Some((LineKey::new(Axis::V, v), u))
    } else {
      None
    };
    match boundary {
      Some((line, along)) => {
        let curve = self.line(line);
        curve.curve.eval(param_to_f64(curve.canonical(along)))
      }
      None => self.surface.eval(param_to_f64(u), param_to_f64(v)),
    }
  }
}
