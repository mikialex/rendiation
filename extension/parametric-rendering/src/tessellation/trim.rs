//! Trim loops on the GPU (plan-trim T1): the upload of a `TrimEncoding` (segment pool and u32
//! table, layout in `parametric_tessellation::trim`) and the shader side of its point and
//! rectangle queries, operation by operation like the CPU f32 mirror (`TrimEncoding::winding`,
//! `TrimEncoding::classify_rect`).

use parametric_normalized_model::ParametricNormalizedModel;
use parametric_tessellation::trim::*;

use crate::*;

/// `EncodedSegment` as uploaded: `(p0, p1)` and `(p2, max_u, bitcast<f32>(flags))`.
#[repr(C)]
#[shader_struct(std430)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TrimSegmentGpu {
  pub p01: Vec4<f32>,
  pub p2_max_u_flags: Vec4<f32>,
}

impl TrimSegmentGpu {
  pub fn new(s: &EncodedSegment) -> Self {
    Self {
      p01: Vec4::new(s.p0.x, s.p0.y, s.p1.x, s.p1.y),
      p2_max_u_flags: Vec4::new(s.p2.x, s.p2.y, s.max_u, f32::from_bits(s.flags)),
      ..Zeroable::zeroed()
    }
  }
}

/// `RectClass` in the shader.
pub const TRIM_RECT_OUTSIDE: u32 = 0;
pub const TRIM_RECT_INSIDE: u32 = 1;
pub const TRIM_RECT_PARTIAL: u32 = 2;

pub fn rect_class_code(class: RectClass) -> u32 {
  match class {
    RectClass::Outside => TRIM_RECT_OUTSIDE,
    RectClass::Inside => TRIM_RECT_INSIDE,
    RectClass::Partial => TRIM_RECT_PARTIAL,
  }
}

/// The trims of a whole model (indexed by `ParametricNormalizedModel::patches`) on the GPU.
#[derive(Clone)]
pub struct TrimDeviceData {
  pub segments: StorageBufferReadonlyDataView<[TrimSegmentGpu]>,
  pub table: StorageBufferReadonlyDataView<[u32]>,
}

/// The default trim encoding of a model: quadratics within the model tolerance, uniform bands
/// plus the pre classification grid.
pub fn encode_model_trims(model: &ParametricNormalizedModel) -> TrimEncoding {
  encode_trims(
    model,
    &TrimQuadraticConfig::for_model(model),
    &BandGrid::DEFAULT,
  )
}

impl TrimDeviceData {
  pub fn new(device: &GPUDevice, encoding: &TrimEncoding) -> Self {
    let mut segments: Vec<_> = encoding.segments.iter().map(TrimSegmentGpu::new).collect();
    if segments.is_empty() {
      segments.push(Default::default());
    }
    let mut table = encoding.table.clone();
    if table.is_empty() {
      table.push(0);
    }
    Self {
      segments: create_gpu_readonly_storage(segments.as_slice(), device, "trim segments"),
      table: create_gpu_readonly_storage(table.as_slice(), device, "trim table"),
    }
  }

  pub fn bind_shader(&self, binding: &mut ShaderBindGroupBuilder) -> ShaderTrimData {
    ShaderTrimData {
      segments: binding.bind_by(&self.segments),
      table: binding.bind_by(&self.table),
    }
  }

  pub fn bind_pass(&self, binding: &mut BindingBuilder) {
    binding.bind(&self.segments);
    binding.bind(&self.table);
  }
}

/// The shader side of `TrimDeviceData`.
#[derive(Clone)]
pub struct ShaderTrimData {
  pub segments: ShaderReadonlyPtrOf<[TrimSegmentGpu]>,
  pub table: ShaderReadonlyPtrOf<[u32]>,
}

/// `PatchTrimHeader` in the shader.
#[derive(Clone, Copy)]
pub struct ShaderTrimHeader {
  pub segment_offset: Node<u32>,
  pub segment_count: Node<u32>,
  pub accel_offset: Node<u32>,
  pub kind: Node<u32>,
}

/// One segment relative to nothing (as stored).
#[derive(Clone, Copy)]
pub struct ShaderTrimSegment {
  pub p0: Node<Vec2<f32>>,
  pub p1: Node<Vec2<f32>>,
  pub p2: Node<Vec2<f32>>,
  pub max_u: Node<f32>,
  pub flags: Node<u32>,
}

impl ShaderTrimSegment {
  pub fn min(&self) -> Node<Vec2<f32>> {
    self.p0.min(self.p1).min(self.p2)
  }

  pub fn max(&self) -> Node<Vec2<f32>> {
    let y = self.p0.y().max(self.p1.y()).max(self.p2.y());
    (self.max_u, y).into()
  }

  /// `EncodedSegment::box_intersects` of the closed rectangle.
  pub fn box_intersects(&self, min: Node<Vec2<f32>>, max: Node<Vec2<f32>>) -> Node<bool> {
    let (smin, smax) = (self.min(), self.max());
    smin
      .x()
      .less_equal_than(max.x())
      .and(smax.x().greater_equal_than(min.x()))
      .and(smin.y().less_equal_than(max.y()))
      .and(smax.y().greater_equal_than(min.y()))
  }

  /// `EncodedSegment::on_domain_boundary`.
  pub fn on_domain_boundary(&self) -> Node<bool> {
    let all = |side: f32, c: fn(Node<Vec2<f32>>) -> Node<f32>| {
      c(self.p0)
        .equals(val(side))
        .and(c(self.p1).equals(val(side)))
        .and(c(self.p2).equals(val(side)))
    };
    let x = |p: Node<Vec2<f32>>| p.x();
    let y = |p: Node<Vec2<f32>>| p.y();
    all(0., x).or(all(1., x)).or(all(0., y)).or(all(1., y))
  }
}

impl ShaderTrimData {
  pub fn header(&self, patch: Node<u32>) -> ShaderTrimHeader {
    let base = patch * val(TRIM_HEADER_WORDS as u32);
    let word = |i: u32| self.table.index(base + val(i)).load();
    ShaderTrimHeader {
      segment_offset: word(0),
      segment_count: word(1),
      accel_offset: word(2),
      kind: word(3),
    }
  }

  pub fn segment(&self, header: &ShaderTrimHeader, local: Node<u32>) -> ShaderTrimSegment {
    let s = self
      .segments
      .index(header.segment_offset + local)
      .load()
      .expand();
    ShaderTrimSegment {
      p0: s.p01.xy(),
      p1: s.p01.zw(),
      p2: s.p2_max_u_flags.xy(),
      max_u: s.p2_max_u_flags.z(),
      flags: s.p2_max_u_flags.w().bitcast::<u32>(),
    }
  }

  /// `TrimEncoding::winding`: the winding number of the (clamped) sample, only `!= 0` is
  /// meaningful for the constant kinds and cells.
  pub fn winding(&self, patch: Node<u32>, uv: Node<Vec2<f32>>) -> Node<i32> {
    let uv = shader_clamp_uv(uv);
    let header = self.header(patch);
    let result = val(1i32).make_local_var();
    switch_by(header.kind)
      .case(TRIM_KIND_OUTSIDE, || result.store(val(0)))
      .case(TRIM_KIND_ALL_SEGMENTS, || {
        result.store(self.winding_all(&header, uv))
      })
      .case(TRIM_KIND_BAND_GRID, || {
        result.store(self.band_grid_winding(&header, uv))
      })
      .end_with_default(|| {});
    result.load()
  }

  /// `PatchTrimView::winding_all`, `uv` clamped.
  pub fn winding_all(&self, header: &ShaderTrimHeader, uv: Node<Vec2<f32>>) -> Node<i32> {
    let winding = val(0i32).make_local_var();
    header.segment_count.into_shader_iter().for_each(|i, _| {
      let s = self.segment(header, i);
      winding.store(winding.load() + shader_trim_crossing(&s, uv));
    });
    winding.load()
  }

  /// `BandGrid::point_query` plus the kernel, `uv` clamped.
  fn band_grid_winding(&self, header: &ShaderTrimHeader, uv: Node<Vec2<f32>>) -> Node<i32> {
    let layout = self.band_grid_layout(header);
    let n = (val(1u32) << layout.grid_log2).into_f32();
    let class = layout.grid_cell(self, (uv.x() * n).into_u32(), (uv.y() * n).into_u32());
    let result = val(1i32).make_local_var();
    if_by(class.equals(val(GRID_OUTSIDE)), || result.store(val(0)))
      .else_if(class.equals(val(GRID_UNDETERMINED)), || {
        let winding = val(0i32).make_local_var();
        let nb = (val(1u32) << layout.band_log2).into_f32();
        let band = (uv.y() * nb).into_u32();
        layout.band(self, band).for_each(|index, cx| {
          let s = self.segment(header, index);
          if_by(s.max_u.less_than(uv.x()), || cx.do_break());
          winding.store(winding.load() + shader_trim_crossing(&s, uv));
        });
        result.store(winding.load());
      })
      .else_by(|| {});
    result.load()
  }

  fn band_grid_layout(&self, header: &ShaderTrimHeader) -> ShaderBandGridLayout {
    let base = header.accel_offset;
    let config = self.table.index(base).load();
    let band_log2 = config & val(0xff);
    let grid_log2 = (config >> val(8)) & val(0xff);
    let n = val(1u32) << grid_log2;
    let band_base = base + val(1) + (n * n + val(15)) / val(16);
    ShaderBandGridLayout {
      base,
      band_log2,
      grid_log2,
      band_base,
      list_base: band_base + (val(1u32) << band_log2),
    }
  }

  /// `TrimEncoding::classify_rect` of the closed rectangle, a `TRIM_RECT_*` code.
  pub fn classify_rect(
    &self,
    patch: Node<u32>,
    min: Node<Vec2<f32>>,
    max: Node<Vec2<f32>>,
  ) -> Node<u32> {
    let header = self.header(patch);
    let result = val(TRIM_RECT_INSIDE).make_local_var();
    switch_by(header.kind)
      .case(TRIM_KIND_OUTSIDE, || result.store(val(TRIM_RECT_OUTSIDE)))
      .case(TRIM_KIND_ALL_SEGMENTS, || {
        let partial = val(false).make_local_var();
        header.segment_count.into_shader_iter().for_each(|i, cx| {
          let s = self.segment(&header, i);
          if_by(
            s.on_domain_boundary().not().and(s.box_intersects(min, max)),
            || {
              partial.store(val(true));
              cx.do_break();
            },
          );
        });
        result.store(self.rect_class_by_center(&header, partial.load(), min, max));
      })
      .case(TRIM_KIND_BAND_GRID, || {
        result.store(self.band_grid_classify_rect(&header, min, max))
      })
      .end_with_default(|| {});
    result.load()
  }

  /// `BandGrid::rect_query`.
  fn band_grid_classify_rect(
    &self,
    header: &ShaderTrimHeader,
    min: Node<Vec2<f32>>,
    max: Node<Vec2<f32>>,
  ) -> Node<u32> {
    let layout = self.band_grid_layout(header);
    let (xs, ys) = (
      cell_range(min.x(), max.x(), layout.grid_log2),
      cell_range(min.y(), max.y(), layout.grid_log2),
    );
    let first = layout.grid_cell(self, xs.0, ys.0);
    let constant = first.not_equals(val(GRID_UNDETERMINED)).make_local_var();
    if_by(constant.load(), || {
      (ys.0..ys.1).into_shader_iter().for_each(|cy, y_cx| {
        (xs.0..xs.1).into_shader_iter().for_each(|cx, x_cx| {
          if_by(layout.grid_cell(self, cx, cy).not_equals(first), || {
            constant.store(val(false));
            x_cx.do_break();
          });
        });
        if_by(constant.load().not(), || y_cx.do_break());
      });
    });
    let result = val(TRIM_RECT_PARTIAL).make_local_var();
    if_by(constant.load(), || {
      let inside = first.equals(val(GRID_INSIDE));
      result.store(inside.select(val(TRIM_RECT_INSIDE), val(TRIM_RECT_OUTSIDE)));
    })
    .else_by(|| {
      let partial = val(false).make_local_var();
      let bands = cell_range(min.y(), max.y(), layout.band_log2);
      (bands.0..bands.1).into_shader_iter().for_each(|b, b_cx| {
        layout.band(self, b).for_each(|index, s_cx| {
          let s = self.segment(header, index);
          if_by(
            s.on_domain_boundary().not().and(s.box_intersects(min, max)),
            || {
              partial.store(val(true));
              s_cx.do_break();
            },
          );
        });
        if_by(partial.load(), || b_cx.do_break());
      });
      result.store(self.rect_class_by_center(header, partial.load(), min, max));
    });
    result.load()
  }

  /// `PatchTrimView::classify_by_boxes` after the box test: partial, or the class of the
  /// center by the kernel over all segments.
  fn rect_class_by_center(
    &self,
    header: &ShaderTrimHeader,
    partial: Node<bool>,
    min: Node<Vec2<f32>>,
    max: Node<Vec2<f32>>,
  ) -> Node<u32> {
    let result = val(TRIM_RECT_PARTIAL).make_local_var();
    if_by(partial.not(), || {
      let center = shader_clamp_uv((min + max) * val(0.5));
      let inside = self.winding_all(header, center).not_equals(val(0));
      result.store(inside.select(val(TRIM_RECT_INSIDE), val(TRIM_RECT_OUTSIDE)));
    });
    result.load()
  }
}

/// `BandGridLayout` in the shader, all offsets absolute in the table.
struct ShaderBandGridLayout {
  base: Node<u32>,
  band_log2: Node<u32>,
  grid_log2: Node<u32>,
  band_base: Node<u32>,
  list_base: Node<u32>,
}

impl ShaderBandGridLayout {
  fn grid_cell(&self, trim: &ShaderTrimData, cx: Node<u32>, cy: Node<u32>) -> Node<u32> {
    let cell = (cy << self.grid_log2) + cx;
    let word = trim.table.index(self.base + val(1) + cell / val(16)).load();
    (word >> (val(2) * (cell % val(16)))) & val(3)
  }

  /// The local segment indices of band `b`.
  fn band(
    &self,
    trim: &ShaderTrimData,
    b: Node<u32>,
  ) -> impl ShaderIterator<Item = Node<u32>> + use<> {
    let word = trim.table.index(self.band_base + b).load();
    let start = self.list_base + (word & val(0xffff));
    let table = trim.table.clone();
    (start..start + (word >> val(16)))
      .into_shader_iter()
      .map(move |i| table.index(i).load())
  }
}

/// The `first..end` cell range of `[min, max]` in `1 << log2` cells, like the CPU `cells`.
fn cell_range(min: Node<f32>, max: Node<f32>, log2: Node<u32>) -> (Node<u32>, Node<u32>) {
  let n = (val(1u32) << log2).into_f32();
  let last = (val(1u32) << log2) - val(1);
  let first = (min.max(val(0.)) * n).into_u32().min(last);
  let end = (max.min(val(1.)) * n)
    .ceil()
    .into_u32()
    .clamp(first + val(1), last + val(1));
  (first, end)
}

/// `clamp_uv`.
pub fn shader_clamp_uv(uv: Node<Vec2<f32>>) -> Node<Vec2<f32>> {
  uv.clamp(
    val(Vec2::splat(UV_CLAMP_MIN)),
    val(Vec2::splat(UV_CLAMP_MAX)),
  )
}

/// `crossing`: the signed crossing count of the `+u` ray from `uv` with the segment, see the
/// CPU version for the root code and the cancellation free roots.
pub fn shader_trim_crossing(s: &ShaderTrimSegment, uv: Node<Vec2<f32>>) -> Node<i32> {
  let result = val(0i32).make_local_var();
  if_by(s.max_u.greater_equal_than(uv.x()), || {
    let (p0, p1, p2) = (s.p0 - uv, s.p1 - uv, s.p2 - uv);
    let code = shader_root_code(p0.y(), p1.y(), p2.y());
    if_by(code.not_equals(val(0)), || {
      let a = p0 - p1 * val(2.) + p2;
      let b = p0 - p1;
      let discriminant = b.y() * b.y() - a.y() * p0.y();
      let t1 = val(0.).make_local_var();
      let t2 = val(0.).make_local_var();
      if_by(discriminant.greater_than(val(0.)), || {
        let d = discriminant.sqrt();
        let positive = b.y().greater_equal_than(val(0.));
        let negative = b.y().less_equal_than(val(0.));
        t1.store(positive.select_branched(|| p0.y() / (b.y() + d), || (b.y() - d) / a.y()));
        t2.store(negative.select_branched(|| p0.y() / (b.y() - d), || (b.y() + d) / a.y()));
      })
      .else_by(|| {
        let t = a
          .y()
          .not_equals(val(0.))
          .select_branched(|| b.y() / a.y(), || val(0.));
        t1.store(t);
        t2.store(t);
      });
      let x_at = |t: Node<f32>| (a.x() * t - b.x() * val(2.)) * t + p0.x();
      let winding = val(0i32).make_local_var();
      if_by(
        (code & val(1))
          .not_equals(val(0))
          .and(x_at(t1.load()).greater_than(val(0.))),
        || winding.store(winding.load() - val(1)),
      );
      if_by(
        code
          .greater_than(val(1))
          .and(x_at(t2.load()).greater_than(val(0.))),
        || winding.store(winding.load() + val(1)),
      );
      result.store(winding.load());
    });
  });
  result.load()
}

/// `root_code` from the sign bits of the sample relative v coordinates.
pub fn shader_root_code(y0: Node<f32>, y1: Node<f32>, y2: Node<f32>) -> Node<u32> {
  let sign = |y: Node<f32>| y.bitcast::<u32>() >> val(31);
  let shift = sign(y0) | (sign(y1) << val(1)) | (sign(y2) << val(2));
  (val(0x2E74u32) >> shift) & val(0x0101)
}
