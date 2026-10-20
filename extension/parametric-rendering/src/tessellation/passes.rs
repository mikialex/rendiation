//! The compute passes of one frame (plan "GPU 调度", M3 and M4), all sizes driven by indirect
//! arguments written on the GPU, no CPU readback inside a frame:
//!
//! T transform, I init, per level L.a extract (per work item and control point), L.b interior
//! criteria (per work item), L.c edge criteria (per work item and edge), L.d decide (per work
//! item: append the children, or output the leaf with its bucket slot and vertex range). The
//! indirect arguments of the next level are maxed atomically by L.d while appending, which
//! replaces the single thread L.p pass of the plan. D finalizes the draw arguments and the
//! vertex dispatch. V: forward position request rounds, backward interpolation rounds, and the
//! final vertex pass (interior positions and all normals).
//!
//! Every append is capacity checked, an overflow drops the write and sets a flag in the
//! counters (read back asynchronously, the buffers grow for a later frame).

use parametric_tessellation::{Camera, PARAM_BITS, PARAM_ONE, TessellationConfig};

use super::*;
use crate::*;

pub const COUNTER_LEAVES: u32 = 0;
pub const COUNTER_VERTICES: u32 = 1;
pub const COUNTER_OVERFLOW: u32 = 2;
pub const COUNTER_CULLED: u32 = 3;
pub const COUNTER_FORCED_LEAVES: u32 = 4;
pub const COUNTER_EDGE_RAISED: u32 = 5;
/// Split counts by `SplitReason` (near plane, interior, size, edge).
pub const COUNTER_SPLITS: u32 = 6;
/// Leaves per `k` bucket.
pub const COUNTER_BUCKETS: u32 = 10;
/// Vertex dispatch prefix per bucket, `K_BUCKET_COUNT + 1` entries, written by D.
pub const COUNTER_VERTEX_PREFIX: u32 = 15;
/// Work items appended per level (requested, may exceed the capacity).
pub const COUNTER_WORK_ITEMS: u32 = 24;
pub const COUNTER_LEVEL_LEAVES: u32 = 48;
/// Position requests appended per round (requested, may exceed the capacity).
pub const COUNTER_ROUNDS: u32 = 72;
pub const COUNTER_COUNT: u32 = 96;
/// Levels and rounds are limited by `PARAM_BITS`.
pub const MAX_STEPS: u32 = 21;

pub const OVERFLOW_QUEUE: u32 = 1;
pub const OVERFLOW_LEAVES: u32 = 2;
pub const OVERFLOW_VERTICES: u32 = 4;
pub const OVERFLOW_REQUESTS: u32 = 8;
/// A position request still inside a piece after the last round (should not happen).
pub const OVERFLOW_ROUNDS: u32 = 16;

/// Interior criteria result codes, `code << 8 | log2 k_grid`.
const STATUS_OK: u32 = 0;
const STATUS_CULL: u32 = 1;
/// Followed by `STATUS_SPLIT_NEAR + SplitReason as u32` for the near plane, interior and size.
const STATUS_SPLIT_NEAR: u32 = 2;
const SPLIT_REASON_EDGE: u32 = 3;
const EDGE_FAIL: u32 = u32::MAX;
const RECORD_INTERPOLATE: u32 = 1;

/// The level or round of a per step dispatch.
#[repr(C)]
#[shader_struct(std140)]
#[derive(Debug, Clone, Copy)]
pub struct TessStepUniform {
  pub step: u32,
}

type Ro<T> = StorageBufferReadonlyDataView<[T]>;
type Rw<T> = StorageBufferDataView<[T]>;
type Atomics = Rw<DeviceAtomic<u32>>;
type Out<K, T> = <K as BindKind>::Out<T>;
type UniformOut<K> = Out<K, UniformBufferDataView<TessUniform>>;
type StepOut<K> = Out<K, UniformBufferDataView<TessStepUniform>>;

/// The per frame buffers.
pub struct TessFrameBuffers {
  pub capacity: TessCapacity,
  pub levels: u32,
  pub rounds: u32,
  pub uniform: UniformBufferDataView<TessUniform>,
  pub steps: Vec<UniformBufferDataView<TessStepUniform>>,
  /// Ping-pong work queues of `(item, level, ix, iy)`.
  pub queues: [Rw<Vec4<u32>>; 2],
  /// `sub_patch_stride` control points per work item.
  pub sub_patches: Rw<Vec4<f32>>,
  pub interior: Rw<u32>,
  /// Four per work item.
  pub edges: Rw<u32>,
  /// Per level, two dispatch arguments: `(x, sub_patch_stride, 1)` for L.a, `(x, 1, 1)`.
  pub level_args: Vec<Atomics>,
  pub counters: Atomics,
  /// `(item, level, ix, iy)` of every leaf.
  pub leaf_keys: Rw<Vec4<u32>>,
  /// Dispatch over all leaf vertices, written by D.
  pub vertex_args: Rw<u32>,
  /// Per round, the dispatch over its requests (round 0 uses `vertex_args`).
  pub round_args: Vec<Atomics>,
  /// `(item, u, v, 0)` of the requests of the rounds after round 0.
  pub requests: Rw<Vec4<u32>>,
  /// Per request (round 0 at the vertex index, then `vertex_capacity + request index`):
  /// `(t bits, child a, child b, RECORD_INTERPOLATE)` or zero for an exact value.
  pub records: Rw<Vec4<u32>>,
  pub values: Rw<Vec4<f32>>,
  pub output: TessellationFrameOutput,
}

fn indirect_buffer<T: Std430 + ShaderSizedValueNodeType>(
  device: &GPUDevice,
  words: usize,
) -> Rw<T> {
  let usage = BufferUsages::INDIRECT
    | BufferUsages::STORAGE
    | BufferUsages::COPY_DST
    | BufferUsages::COPY_SRC;
  let zeros = vec![0u32; words];
  let buffer = create_gpu_buffer(cast_slice(&zeros), usage, device).create_default_view();
  StorageBufferDataView::try_from_raw(buffer).unwrap()
}

fn zeroed<T: Std430 + ShaderSizedValueNodeType>(
  device: &GPUDevice,
  len: u32,
  label: &str,
) -> Rw<T> {
  create_gpu_read_write_storage::<[T]>(ZeroedArrayByArrayLength(len.max(1) as usize), device, label)
}

fn raw_buffer(view: &GPUBufferResourceView) -> &raw_gpu::Buffer {
  view.resource.gpu_resource().gpu()
}

impl TessFrameBuffers {
  pub fn new(
    device: &GPUDevice,
    capacity: TessCapacity,
    config: &TessellationConfig,
    sub_patch_stride: u32,
  ) -> Self {
    let levels = config.max_level + 1;
    let rounds = config.max_level + config.k_max.trailing_zeros();
    assert!(levels <= MAX_STEPS && rounds < MAX_STEPS);
    let queue = capacity.queue;
    let requests = capacity.requests;
    let records = capacity.vertices + requests;
    Self {
      capacity,
      levels,
      rounds,
      uniform: create_uniform(
        TessUniform::new(&identity_camera(), config),
        device,
        "tessellation uniform",
      ),
      steps: (0..=MAX_STEPS)
        .map(|step| {
          create_uniform(
            TessStepUniform {
              step,
              ..Zeroable::zeroed()
            },
            device,
            "tessellation step",
          )
        })
        .collect(),
      queues: [
        zeroed(device, queue, "tessellation queue a"),
        zeroed(device, queue, "tessellation queue b"),
      ],
      sub_patches: zeroed(device, queue * sub_patch_stride, "tessellation sub patches"),
      interior: zeroed(device, queue, "tessellation interior"),
      edges: zeroed(device, queue * 4, "tessellation edges"),
      level_args: (0..=levels).map(|_| indirect_buffer(device, 6)).collect(),
      counters: zeroed::<u32>(device, COUNTER_COUNT, "tessellation counters")
        .into_device_atomic_array(),
      leaf_keys: zeroed(device, capacity.leaves, "tessellation leaf keys"),
      vertex_args: indirect_buffer(device, 3),
      round_args: (0..=rounds + 1)
        .map(|_| indirect_buffer(device, 3))
        .collect(),
      requests: zeroed(device, requests, "tessellation requests"),
      records: zeroed(device, records, "tessellation records"),
      values: zeroed(device, records, "tessellation values"),
      output: TessellationFrameOutput::new(device, capacity.vertices, capacity.leaves),
    }
  }

  /// Records all passes of a frame. `uniform` must already hold this frame's values.
  pub fn encode(&self, device: &GPUDevice, encoder: &mut GPUCommandEncoder, data: &TessDeviceData) {
    encoder.clear_buffer(raw_buffer(&self.counters), 0, None);
    for args in self.level_args.iter().chain(&self.round_args) {
      encoder.clear_buffer(raw_buffer(args), 0, None);
    }
    let cx = |step: u32| PassCtx {
      data,
      frame: self,
      step,
    };
    encoder.compute_pass_scoped(|mut pass| {
      let pass = &mut pass;
      let camera_cps = data.camera_cps.item_count();
      let items = data.items.item_count();
      encode::<TransformPass>(&cx(0), device, pass, Dispatch::direct(camera_cps, 256));
      encode::<InitPass>(&cx(0), device, pass, Dispatch::direct(items, 64));
      for level in 0..self.levels {
        let args = &self.level_args[level as usize].gpu;
        let cx = cx(level);
        encode::<ExtractPass>(&cx, device, pass, Dispatch::Indirect(args, 0));
        encode::<InteriorPass>(&cx, device, pass, Dispatch::Indirect(args, 12));
        encode::<EdgePass>(&cx, device, pass, Dispatch::Indirect(args, 12));
        encode::<DecidePass>(&cx, device, pass, Dispatch::Indirect(args, 12));
      }
      encode::<FinalizePass>(&cx(0), device, pass, Dispatch::Direct(1));
      let round_dispatch = |round: u32| {
        if round == 0 {
          Dispatch::Indirect(&self.vertex_args.gpu, 0)
        } else {
          Dispatch::Indirect(&self.round_args[round as usize].gpu, 0)
        }
      };
      for round in 0..=self.rounds {
        encode::<ForwardPass>(&cx(round), device, pass, round_dispatch(round));
      }
      for round in (0..self.rounds).rev() {
        encode::<BackwardPass>(&cx(round), device, pass, round_dispatch(round));
      }
      encode::<VertexPass>(&cx(0), device, pass, round_dispatch(0));
    });
  }

  /// `encode` dispatch count.
  pub fn dispatch_count(&self) -> u32 {
    3 + self.levels * 4 + (self.rounds + 1) + self.rounds + 1
  }
}

fn identity_camera() -> Camera {
  Camera {
    view: Mat4::identity(),
    focal_length: 1.,
    viewport: Vec2::new(1., 1.),
    near: 1.,
  }
}

struct PassCtx<'a> {
  data: &'a TessDeviceData,
  frame: &'a TessFrameBuffers,
  step: u32,
}

impl PassCtx<'_> {
  fn step(&self) -> &UniformBufferDataView<TessStepUniform> {
    &self.frame.steps[self.step as usize]
  }

  fn queue_in(&self) -> &Rw<Vec4<u32>> {
    &self.frame.queues[self.step as usize % 2]
  }

  fn queue_out(&self) -> &Rw<Vec4<u32>> {
    &self.frame.queues[(self.step as usize + 1) % 2]
  }
}

enum Dispatch<'a> {
  Direct(u32),
  Indirect(&'a GPUBufferResourceView, u64),
}

impl Dispatch<'_> {
  fn direct(threads: u32, workgroup: u32) -> Self {
    Self::Direct(threads.div_ceil(workgroup))
  }
}

/// One compute pass: its bindings (declared once for the shader and the pass side) and its
/// shader.
trait TessPass: 'static {
  const WORKGROUP: (u32, u32, u32);
  type Bindings<K: BindKind>;
  fn declare<B: Binder>(cx: &PassCtx, b: &mut B) -> Self::Bindings<B::Kind>;
  fn build(builder: &ShaderComputePipelineBuilder, b: Self::Bindings<ShaderKind>);
}

fn encode<P: TessPass>(
  cx: &PassCtx,
  device: &GPUDevice,
  pass: &mut GPUComputePass,
  dispatch: Dispatch,
) {
  let hasher = PipelineHasher::default().with_hash_type::<P>();
  let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
    builder = builder.with_config_work_group_size(P::WORKGROUP);
    let bindings = P::declare(cx, &mut ShaderBinder(builder.bindgroups()));
    P::build(&builder, bindings);
    builder
  });
  let mut binding = BindingBuilder::default();
  P::declare(cx, &mut PassBinder(&mut binding));
  binding.setup_compute_pass(pass, device, &pipeline);
  match dispatch {
    Dispatch::Direct(x) => {
      if x > 0 {
        pass.dispatch_workgroups(x, 1, 1)
      }
    }
    Dispatch::Indirect(args, offset) => {
      pass.dispatch_workgroups_indirect(raw_buffer(args), args.desc.offset + offset)
    }
  }
}

fn atomic(
  counters: &ShaderPtrOf<[DeviceAtomic<u32>]>,
  index: u32,
) -> ShaderPtrOf<DeviceAtomic<u32>> {
  counters.index(val(index))
}

fn set_overflow(counters: &ShaderPtrOf<[DeviceAtomic<u32>]>, flag: u32) {
  atomic(counters, COUNTER_OVERFLOW).atomic_or(val(flag));
}

/// Work items of the current level, clamped to the queue capacity.
fn level_count(
  u: &ENode<TessUniform>,
  counters: &ShaderPtrOf<[DeviceAtomic<u32>]>,
  level: Node<u32>,
) -> Node<u32> {
  let count = counters
    .index(val(COUNTER_WORK_ITEMS) + level)
    .atomic_load();
  count.min(u.queue_capacity)
}

/// A work item key `(item, level, ix, iy)`.
#[derive(Clone, Copy)]
struct ShaderKey {
  item: Node<u32>,
  level: Node<u32>,
  ix: Node<u32>,
  iy: Node<u32>,
}

impl ShaderKey {
  fn new(key: Node<Vec4<u32>>) -> Self {
    Self {
      item: key.x(),
      level: key.y(),
      ix: key.z(),
      iy: key.w(),
    }
  }

  fn u_segment(&self) -> ShaderSegment {
    ShaderSegment::new(self.level, self.ix)
  }

  fn v_segment(&self) -> ShaderSegment {
    ShaderSegment::new(self.level, self.iy)
  }
}

/// Reads the camera space control points of an item.
fn item_reader<'a>(
  camera_cps: &'a ShaderPtrOf<[Vec4<f32>]>,
  item: &ENode<TessItem>,
) -> impl Fn(Node<u32>, Node<u32>) -> Node<Vec4<f32>> + 'a {
  let (offset, row) = (item.camera_cp_offset, item.degree_u + val(1));
  move |i, j| camera_cps.index(offset + j * row + i).load()
}

struct TransformPass;
impl TessPass for TransformPass {
  const WORKGROUP: (u32, u32, u32) = (256, 1, 1);
  type Bindings<K: BindKind> = (
    UniformOut<K>,
    Out<K, Ro<Vec4<f32>>>,
    Out<K, Ro<Vec2<u32>>>,
    Out<K, Ro<Mat4<f32>>>,
    Out<K, Rw<Vec4<f32>>>,
  );
  fn declare<B: Binder>(cx: &PassCtx, b: &mut B) -> Self::Bindings<B::Kind> {
    (
      b.bind(&cx.frame.uniform),
      b.bind(&cx.data.cp_pool),
      b.bind(&cx.data.cp_sources),
      b.bind(&cx.data.matrices),
      b.bind(&cx.data.camera_cps),
    )
  }
  fn build(
    builder: &ShaderComputePipelineBuilder,
    (u, pool, sources, matrices, camera): Self::Bindings<ShaderKind>,
  ) {
    let u = u.load().expand();
    let id = builder.global_invocation_id().x();
    if_by(id.greater_equal_than(u.camera_cp_count), do_return);
    let source = sources.index(id).load();
    let matrix = matrices.index(source.y()).load();
    camera
      .index(id)
      .store(matrix * pool.index(source.x()).load());
  }
}

struct InitPass;
impl TessPass for InitPass {
  const WORKGROUP: (u32, u32, u32) = (64, 1, 1);
  type Bindings<K: BindKind> = (
    UniformOut<K>,
    Out<K, Rw<Vec4<u32>>>,
    Out<K, Atomics>,
    Out<K, Atomics>,
  );
  fn declare<B: Binder>(cx: &PassCtx, b: &mut B) -> Self::Bindings<B::Kind> {
    (
      b.bind(&cx.frame.uniform),
      b.bind(&cx.frame.queues[0]),
      b.bind(&cx.frame.counters),
      b.bind(&cx.frame.level_args[0]),
    )
  }
  fn build(
    builder: &ShaderComputePipelineBuilder,
    (u, queue, counters, args): Self::Bindings<ShaderKind>,
  ) {
    let u = u.load().expand();
    let id = builder.global_invocation_id().x();
    let roots = u.item_count.min(u.queue_capacity);
    if_by(id.equals(val(0)).and(roots.less_than(u.item_count)), || {
      set_overflow(&counters, OVERFLOW_QUEUE);
    });
    if_by(id.greater_equal_than(roots), do_return);
    queue.index(id).store(uvec4(id, val(0), val(0), val(0)));
    if_by(id.equals(val(0)), || {
      atomic(&counters, COUNTER_WORK_ITEMS).atomic_store(roots);
      let x = (roots + val(SUBDIVISION_WORKGROUP - 1)) / val(SUBDIVISION_WORKGROUP);
      let values = [x, u.sub_patch_stride, val(1), x, val(1), val(1)];
      for (i, v) in values.into_iter().enumerate() {
        args.index(val(i as u32)).atomic_store(v);
      }
    });
  }
}

/// L.a: one thread per (work item, sub patch control point).
struct ExtractPass;
impl TessPass for ExtractPass {
  const WORKGROUP: (u32, u32, u32) = (SUBDIVISION_WORKGROUP, 1, 1);
  type Bindings<K: BindKind> = (
    UniformOut<K>,
    StepOut<K>,
    Out<K, Ro<TessItem>>,
    Out<K, Rw<Vec4<f32>>>,
    Out<K, Rw<Vec4<u32>>>,
    Out<K, Atomics>,
    Out<K, Rw<Vec4<f32>>>,
  );
  fn declare<B: Binder>(cx: &PassCtx, b: &mut B) -> Self::Bindings<B::Kind> {
    (
      b.bind(&cx.frame.uniform),
      b.bind(cx.step()),
      b.bind(&cx.data.items),
      b.bind(&cx.data.camera_cps),
      b.bind(cx.queue_in()),
      b.bind(&cx.frame.counters),
      b.bind(&cx.frame.sub_patches),
    )
  }
  fn build(
    builder: &ShaderComputePipelineBuilder,
    (u, step, items, camera_cps, queue, counters, sub_patches): Self::Bindings<ShaderKind>,
  ) {
    let u = u.load().expand();
    let level = step.load().expand().step;
    let id = builder.global_invocation_id();
    let (w, c) = (id.x(), id.y());
    if_by(
      w.greater_equal_than(level_count(&u, &counters, level)),
      do_return,
    );
    let key = ShaderKey::new(queue.index(w).load());
    let item = items.index(key.item).load().expand();
    let (n, m) = (item.degree_u, item.degree_v);
    if_by(c.greater_equal_than((n + val(1)) * (m + val(1))), do_return);
    let (i, j) = (c % (n + val(1)), c / (n + val(1)));
    let (us, vs) = (key.u_segment(), key.v_segment());
    let cp = item_reader(&camera_cps, &item);
    let p = sub_patch_point(
      &cp,
      n,
      m,
      i,
      j,
      (param_to_f32(us.start()), param_to_f32(us.end())),
      (param_to_f32(vs.start()), param_to_f32(vs.end())),
    );
    sub_patches.index(w * u.sub_patch_stride + c).store(p);
  }
}

/// L.b: bounds, cull and the grid resolution of the interior and size criteria.
struct InteriorPass;
impl TessPass for InteriorPass {
  const WORKGROUP: (u32, u32, u32) = (SUBDIVISION_WORKGROUP, 1, 1);
  type Bindings<K: BindKind> = (
    UniformOut<K>,
    StepOut<K>,
    Out<K, Ro<TessItem>>,
    Out<K, Rw<Vec4<u32>>>,
    Out<K, Atomics>,
    Out<K, Rw<Vec4<f32>>>,
    Out<K, Rw<u32>>,
  );
  fn declare<B: Binder>(cx: &PassCtx, b: &mut B) -> Self::Bindings<B::Kind> {
    (
      b.bind(&cx.frame.uniform),
      b.bind(cx.step()),
      b.bind(&cx.data.items),
      b.bind(cx.queue_in()),
      b.bind(&cx.frame.counters),
      b.bind(&cx.frame.sub_patches),
      b.bind(&cx.frame.interior),
    )
  }
  fn build(
    builder: &ShaderComputePipelineBuilder,
    (u, step, items, queue, counters, sub_patches, interior): Self::Bindings<ShaderKind>,
  ) {
    let u = u.load().expand();
    let level = step.load().expand().step;
    let w = builder.global_invocation_id().x();
    if_by(
      w.greater_equal_than(level_count(&u, &counters, level)),
      do_return,
    );
    let key = ShaderKey::new(queue.index(w).load());
    let item = items.index(key.item).load().expand();
    let (n, m) = (item.degree_u, item.degree_v);
    let base = w * u.sub_patch_stride;
    let cp = |i: Node<u32>, j: Node<u32>| sub_patches.index(base + j * (n + val(1)) + i).load();
    let bounds = shader_surface_bounds(&cp, n, m);

    let status = val(STATUS_OK).make_local_var();
    let depth = -bounds.max.z();
    if_by(culls(&u, bounds.min, bounds.max), || {
      status.store(val(STATUS_CULL << 8));
    })
    .else_if(depth.less_equal_than(u.near), || {
      status.store(val(STATUS_SPLIT_NEAR << 8));
    })
    .else_by(|| {
      let scale = u.focal_length / depth;
      let tolerance = u.interior_tolerance / scale;
      let k_error = first_power_of_two(&u, |k| bounds.grid_error(k).less_equal_than(tolerance));
      let size = (bounds.max - bounds.min).length() * scale;
      let k_size = first_power_of_two(&u, |k| {
        (size / k.into_f32()).less_equal_than(u.max_triangle_size)
      });
      let split_interior = val((STATUS_SPLIT_NEAR + 1) << 8);
      let split_size = val((STATUS_SPLIT_NEAR + 2) << 8);
      let none = val(u32::MAX);
      let result = k_error.equals(none).select(
        split_interior,
        k_size.equals(none).select(split_size, k_error.max(k_size)),
      );
      status.store(result);
    });
    interior.index(w).store(status.load());
  }
}

/// `log2` of the first `k = 1, 2, 4 .. k_max` accepted by `accept`, `u32::MAX` if none.
fn first_power_of_two(
  u: &ENode<TessUniform>,
  accept: impl Fn(Node<u32>) -> Node<bool>,
) -> Node<u32> {
  let found = val(u32::MAX).make_local_var();
  for_range(val(0), u.log2_k_max + val(1), |e, cx| {
    if_by(accept(val(1u32) << e), || {
      found.store(e);
      cx.do_break();
    });
  });
  found.load()
}

/// L.c: the flat ancestor of each of the four edges, giving the `k` the edge needs.
struct EdgePass;
impl TessPass for EdgePass {
  const WORKGROUP: (u32, u32, u32) = (SUBDIVISION_WORKGROUP, 4, 1);
  type Bindings<K: BindKind> = (
    UniformOut<K>,
    StepOut<K>,
    Out<K, Ro<TessItem>>,
    Out<K, Rw<Vec4<f32>>>,
    Out<K, Rw<Vec4<u32>>>,
    Out<K, Atomics>,
    Out<K, Rw<u32>>,
    Out<K, Rw<u32>>,
  );
  fn declare<B: Binder>(cx: &PassCtx, b: &mut B) -> Self::Bindings<B::Kind> {
    (
      b.bind(&cx.frame.uniform),
      b.bind(cx.step()),
      b.bind(&cx.data.items),
      b.bind(&cx.data.camera_cps),
      b.bind(cx.queue_in()),
      b.bind(&cx.frame.counters),
      b.bind(&cx.frame.interior),
      b.bind(&cx.frame.edges),
    )
  }
  fn build(
    builder: &ShaderComputePipelineBuilder,
    (u, step, items, camera_cps, queue, counters, interior, edges): Self::Bindings<ShaderKind>,
  ) {
    let u = u.load().expand();
    let level = step.load().expand().step;
    let w = builder.global_invocation_id().x();
    let e = builder.local_invocation_id().y();
    if_by(
      w.greater_equal_than(level_count(&u, &counters, level)),
      do_return,
    );
    if_by(
      (interior.index(w).load() >> val(8)).not_equals(val(STATUS_OK)),
      do_return,
    );
    let key = ShaderKey::new(queue.index(w).load());
    let item = items.index(key.item).load().expand();
    let (us, vs) = (key.u_segment(), key.v_segment());
    // Edges in the CPU `SubPatchKey::edges` order: u start, u end, v start, v end.
    let axis_v = e.greater_equal_than(val(2));
    let first = e.equals(val(0)).or(e.equals(val(2)));
    let coord = axis_v.select(
      first.select(vs.start(), vs.end()),
      first.select(us.start(), us.end()),
    );
    let along = ShaderSegment::new(key.level, axis_v.select(us.index, vs.index));
    let cp = item_reader(&camera_cps, &item);
    let points = make_curve_buffer();
    let tmp = make_curve_buffer();
    let curve = load_line_curve(
      &cp,
      item.degree_u,
      item.degree_v,
      axis_v,
      coord,
      &points,
      &tmp,
    );
    let along = curve.canonical_segment(along);
    let (flat, pieces) = flat_ancestor(&u, &points, curve, along, &tmp);
    let k = pieces >> (along.level - flat.level);
    let result = pieces.equals(val(0)).select(val(EDGE_FAIL), k);
    edges.index(w * val(4) + e).store(result);
  }
}

/// L.d: split into four children or output the leaf.
struct DecidePass;
impl TessPass for DecidePass {
  const WORKGROUP: (u32, u32, u32) = (SUBDIVISION_WORKGROUP, 1, 1);
  type Bindings<K: BindKind> = (
    (UniformOut<K>, StepOut<K>, Out<K, Ro<TessItem>>),
    (Out<K, Rw<Vec4<u32>>>, Out<K, Rw<Vec4<u32>>>),
    (Out<K, Rw<u32>>, Out<K, Rw<u32>>),
    (Out<K, Atomics>, Out<K, Atomics>),
    (
      Out<K, Rw<TessLeafDraw>>,
      Out<K, Rw<Vec4<u32>>>,
      Out<K, Rw<u32>>,
    ),
  );
  fn declare<B: Binder>(cx: &PassCtx, b: &mut B) -> Self::Bindings<B::Kind> {
    (
      (
        b.bind(&cx.frame.uniform),
        b.bind(cx.step()),
        b.bind(&cx.data.items),
      ),
      (b.bind(cx.queue_in()), b.bind(cx.queue_out())),
      (b.bind(&cx.frame.interior), b.bind(&cx.frame.edges)),
      (
        b.bind(&cx.frame.counters),
        b.bind(&cx.frame.level_args[cx.step as usize + 1]),
      ),
      (
        b.bind(&cx.frame.output.leaves),
        b.bind(&cx.frame.leaf_keys),
        b.bind(&cx.frame.output.bucket_leaves),
      ),
    )
  }
  fn build(builder: &ShaderComputePipelineBuilder, b: Self::Bindings<ShaderKind>) {
    let ((u, step, items), (queue, queue_out), (interior, edges), (counters, args), leaf_out) = b;
    let (leaves, leaf_keys, bucket_leaves) = leaf_out;
    let u = u.load().expand();
    let level = step.load().expand().step;
    let w = builder.global_invocation_id().x();
    if_by(
      w.greater_equal_than(level_count(&u, &counters, level)),
      do_return,
    );
    let raw_key = queue.index(w).load();
    let key = ShaderKey::new(raw_key);
    let status = interior.index(w).load();
    let code = status >> val(8);
    if_by(code.equals(val(STATUS_CULL)), || {
      atomic(&counters, COUNTER_CULLED).atomic_add(val(1));
      do_return();
    });

    let k_grid = val(1u32) << (status & val(0xff));
    let k = k_grid.make_local_var();
    let edge_fail = val(false).make_local_var();
    if_by(code.equals(val(STATUS_OK)), || {
      for e in 0..4 {
        let ek = edges.index(w * val(4) + val(e)).load();
        if_by(ek.equals(val(EDGE_FAIL)), || edge_fail.store(val(true)))
          .else_by(|| k.store(k.load().max(ek)));
      }
    });
    let leafable = code.equals(val(STATUS_OK)).and(edge_fail.load().not());

    if_by(leafable.not().and(level.less_than(u.max_level)), || {
      let reason = edge_fail
        .load()
        .select(val(SPLIT_REASON_EDGE), code - val(STATUS_SPLIT_NEAR));
      counters
        .index(val(COUNTER_SPLITS) + reason)
        .atomic_add(val(1));
      let index = counters
        .index(val(COUNTER_WORK_ITEMS) + level + val(1))
        .atomic_add(val(4));
      if_by((index + val(4)).greater_than(u.queue_capacity), || {
        set_overflow(&counters, OVERFLOW_QUEUE);
        do_return();
      });
      for (c, (dx, dy)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
        let child = uvec4(
          key.item,
          level + val(1),
          key.ix * val(2) + val(dx),
          key.iy * val(2) + val(dy),
        );
        queue_out.index(index + val(c as u32)).store(child);
      }
      let x = (index + val(4) + val(SUBDIVISION_WORKGROUP - 1)) / val(SUBDIVISION_WORKGROUP);
      args.index(val(0)).atomic_max(x);
      args.index(val(1)).atomic_store(u.sub_patch_stride);
      args.index(val(2)).atomic_store(val(1));
      args.index(val(3)).atomic_max(x);
      args.index(val(4)).atomic_store(val(1));
      args.index(val(5)).atomic_store(val(1));
      do_return();
    });

    // A leaf, forced with `k_max` when the criteria fail at `max_level`.
    let forced = leafable.not();
    let k = forced.select(u.k_max, k.load());
    if_by(forced, || {
      atomic(&counters, COUNTER_FORCED_LEAVES).atomic_add(val(1));
    });
    if_by(leafable.and(k.greater_than(k_grid)), || {
      atomic(&counters, COUNTER_EDGE_RAISED).atomic_add(val(1));
    });
    let size = (k + val(1)) * (k + val(1));
    let vertex_base = atomic(&counters, COUNTER_VERTICES).atomic_add(size);
    // written so that the counter growing past an overflow can not wrap into the capacity
    let vertex_overflow = size
      .greater_than(u.vertex_capacity)
      .or(vertex_base.greater_than(u.vertex_capacity - size));
    if_by(vertex_overflow, || {
      set_overflow(&counters, OVERFLOW_VERTICES);
      do_return();
    });
    let leaf = atomic(&counters, COUNTER_LEAVES).atomic_add(val(1));
    if_by(leaf.greater_equal_than(u.leaf_capacity), || {
      set_overflow(&counters, OVERFLOW_LEAVES);
      do_return();
    });
    let bucket = k.count_trailing_zeros();
    let slot = counters
      .index(val(COUNTER_BUCKETS) + bucket)
      .atomic_add(val(1));
    let item = items.index(key.item).load().expand();
    let trimmed = (item.flags & val(ITEM_FLAG_TRIMMED)).not_equals(val(0));
    let draw = ENode::<TessLeafDraw> {
      vertex_base,
      k,
      item: key.item,
      flags: trimmed.select(val(LEAF_FLAG_TRIMMED), val(0)),
    }
    .construct();
    leaves.index(leaf).store(draw);
    leaf_keys.index(leaf).store(raw_key);
    bucket_leaves
      .index(bucket * u.leaf_capacity + slot)
      .store(leaf);
    counters
      .index(val(COUNTER_LEVEL_LEAVES) + level)
      .atomic_add(val(1));
  }
}

/// D: draw arguments per bucket and the vertex dispatch.
struct FinalizePass;
impl TessPass for FinalizePass {
  const WORKGROUP: (u32, u32, u32) = (1, 1, 1);
  type Bindings<K: BindKind> = (
    UniformOut<K>,
    Out<K, Atomics>,
    Out<K, Rw<DrawIndirectArgsStorage>>,
    Out<K, Rw<u32>>,
  );
  fn declare<B: Binder>(cx: &PassCtx, b: &mut B) -> Self::Bindings<B::Kind> {
    (
      b.bind(&cx.frame.uniform),
      b.bind(&cx.frame.counters),
      b.bind(&cx.frame.output.draw_args),
      b.bind(&cx.frame.vertex_args),
    )
  }
  fn build(
    _: &ShaderComputePipelineBuilder,
    (u, counters, draw_args, vertex_args): Self::Bindings<ShaderKind>,
  ) {
    let u = u.load().expand();
    let mut total = val(0u32);
    for b in 0..K_BUCKET_COUNT as u32 {
      let k = 1u32 << b;
      let count = atomic(&counters, COUNTER_BUCKETS + b)
        .atomic_load()
        .min(u.leaf_capacity);
      let args = ENode::<DrawIndirectArgsStorage> {
        vertex_count: val(6 * k * k),
        instance_count: count,
        base_vertex: val(0),
        base_instance: val(0),
      }
      .construct();
      draw_args.index(val(b)).store(args);
      atomic(&counters, COUNTER_VERTEX_PREFIX + b).atomic_store(total);
      total += count * val((k + 1) * (k + 1));
    }
    atomic(&counters, COUNTER_VERTEX_PREFIX + K_BUCKET_COUNT as u32).atomic_store(total);
    vertex_args
      .index(val(0))
      .store((total + val(VERTEX_WORKGROUP - 1)) / val(VERTEX_WORKGROUP));
    vertex_args.index(val(1)).store(val(1));
    vertex_args.index(val(2)).store(val(1));
  }
}

/// A leaf grid vertex addressed by a thread of the per vertex dispatch (buckets in order).
struct ShaderLeafVertex {
  key: ShaderKey,
  k: Node<u32>,
  i: Node<u32>,
  j: Node<u32>,
  /// Vertex pool index.
  vertex: Node<u32>,
  boundary: Node<bool>,
}

impl ShaderLeafVertex {
  /// The parameter of the leaf center.
  fn center(&self) -> (Node<f32>, Node<f32>) {
    let (us, vs) = (self.key.u_segment(), self.key.v_segment());
    let half = |s: ShaderSegment| (param_to_f32(s.start()) + param_to_f32(s.end())) * val(0.5);
    (half(us), half(vs))
  }

  fn u(&self) -> Node<u32> {
    let us = self.key.u_segment();
    us.start() + self.i * ((val(1u32) << (val(PARAM_BITS) - us.level)) / self.k)
  }

  fn v(&self) -> Node<u32> {
    let vs = self.key.v_segment();
    vs.start() + self.j * ((val(1u32) << (val(PARAM_BITS) - vs.level)) / self.k)
  }
}

/// The vertex of thread `id`, returns from the shader if past the last vertex.
fn leaf_vertex(
  u: &ENode<TessUniform>,
  id: Node<u32>,
  counters: &ShaderPtrOf<[DeviceAtomic<u32>]>,
  bucket_leaves: &ShaderPtrOf<[u32]>,
  leaves: &ShaderPtrOf<[TessLeafDraw]>,
  leaf_keys: &ShaderPtrOf<[Vec4<u32>]>,
) -> ShaderLeafVertex {
  let prefix = |b: u32| atomic(counters, COUNTER_VERTEX_PREFIX + b).atomic_load();
  if_by(
    id.greater_equal_than(prefix(K_BUCKET_COUNT as u32)),
    do_return,
  );
  let mut bucket = val(0u32);
  for b in 1..K_BUCKET_COUNT as u32 {
    bucket = id.greater_equal_than(prefix(b)).select(val(b), bucket);
  }
  let start = counters
    .index(val(COUNTER_VERTEX_PREFIX) + bucket)
    .atomic_load();
  let k = val(1u32) << bucket;
  let row = k + val(1);
  let relative = id - start;
  let slot = relative / (row * row);
  let local = relative % (row * row);
  let leaf = bucket_leaves.index(bucket * u.leaf_capacity + slot).load();
  let draw = leaves.index(leaf).load().expand();
  let key = ShaderKey::new(leaf_keys.index(leaf).load());
  let (i, j) = (local % row, local / row);
  let boundary = i
    .equals(val(0))
    .or(j.equals(val(0)))
    .or(i.equals(k))
    .or(j.equals(k));
  ShaderLeafVertex {
    key,
    k,
    i,
    j,
    vertex: draw.vertex_base + local,
    boundary,
  }
}

/// Requests of round `round` start at `round_start(round)` in the request pool (rounds after 0).
fn round_start(counters: &ShaderPtrOf<[DeviceAtomic<u32>]>, round: Node<u32>) -> Node<u32> {
  let start = val(0u32).make_local_var();
  for_range(val(1), round, |r, _| {
    let count = counters.index(val(COUNTER_ROUNDS) + r).atomic_load();
    start.store(start.load() + count);
  });
  start.load()
}

/// The request pool index of thread `id` in a round after round 0, returns from the shader
/// past the last stored request.
fn round_request(
  u: &ENode<TessUniform>,
  id: Node<u32>,
  round: Node<u32>,
  counters: &ShaderPtrOf<[DeviceAtomic<u32>]>,
) -> Node<u32> {
  let start = round_start(counters, round);
  let count = counters.index(val(COUNTER_ROUNDS) + round).atomic_load();
  let stored = start
    .less_than(u.request_capacity)
    .select(u.request_capacity - start, val(0));
  if_by(id.greater_equal_than(count.min(stored)), do_return);
  start + id
}

/// V forward round: position request `(item, u, v)` inside a linearization piece records the
/// interpolation and requests the piece ends in the next round, any other is evaluated
/// exactly. Round 0 requests are the leaf boundary vertices (request index = vertex index).
struct ForwardPass;
impl TessPass for ForwardPass {
  const WORKGROUP: (u32, u32, u32) = (VERTEX_WORKGROUP, 1, 1);
  type Bindings<K: BindKind> = (
    (
      UniformOut<K>,
      StepOut<K>,
      Out<K, Ro<TessItem>>,
      Out<K, Rw<Vec4<f32>>>,
    ),
    (
      Out<K, Rw<TessLeafDraw>>,
      Out<K, Rw<Vec4<u32>>>,
      Out<K, Rw<u32>>,
      Out<K, Atomics>,
    ),
    (
      Out<K, Rw<Vec4<u32>>>,
      Out<K, Rw<Vec4<u32>>>,
      Out<K, Rw<Vec4<f32>>>,
      Out<K, Atomics>,
    ),
  );
  fn declare<B: Binder>(cx: &PassCtx, b: &mut B) -> Self::Bindings<B::Kind> {
    (
      (
        b.bind(&cx.frame.uniform),
        b.bind(cx.step()),
        b.bind(&cx.data.items),
        b.bind(&cx.data.camera_cps),
      ),
      (
        b.bind(&cx.frame.output.leaves),
        b.bind(&cx.frame.leaf_keys),
        b.bind(&cx.frame.output.bucket_leaves),
        b.bind(&cx.frame.counters),
      ),
      (
        b.bind(&cx.frame.requests),
        b.bind(&cx.frame.records),
        b.bind(&cx.frame.values),
        b.bind(&cx.frame.round_args[cx.step as usize + 1]),
      ),
    )
  }
  fn build(builder: &ShaderComputePipelineBuilder, b: Self::Bindings<ShaderKind>) {
    let ((u, step, items, camera_cps), (leaves, leaf_keys, bucket_leaves, counters), rest) = b;
    let (requests, records, values, args) = rest;
    let u = u.load().expand();
    let round = step.load().expand().step;
    let id = builder.global_invocation_id().x();

    let request = zeroed_val::<Vec4<u32>>().make_local_var();
    let index = val(0u32).make_local_var();
    if_by(round.equals(val(0)), || {
      let v = leaf_vertex(&u, id, &counters, &bucket_leaves, &leaves, &leaf_keys);
      if_by(v.boundary.not(), do_return);
      request.store(uvec4(v.key.item, v.u(), v.v(), val(0)));
      index.store(v.vertex);
    })
    .else_by(|| {
      let r = round_request(&u, id, round, &counters);
      request.store(requests.index(r).load());
      index.store(u.vertex_capacity + r);
    });
    let request = request.load();
    let q = index.load();
    let (item_index, pu, pv) = (request.x(), request.y(), request.z());
    let item = items.index(item_index).load().expand();
    let (n, m) = (item.degree_u, item.degree_v);
    let cp = item_reader(&camera_cps, &item);
    let points = make_curve_buffer();
    let tmp = make_curve_buffer();

    // The piece of the line u = pu first, then v = pv, see the CPU `PatchEval::piece`.
    let found = val(false).make_local_var();
    let t = val(0.).make_local_var();
    let ends = zeroed_val::<Vec4<u32>>().make_local_var();
    for_range(val(0), val(2), |axis, cx| {
      let axis_v = axis.equals(val(1));
      let coord = axis_v.select(pv, pu);
      let along = axis_v.select(pu, pv);
      let curve = load_line_curve(&cp, n, m, axis_v, coord, &points, &tmp);
      let p = curve.canonical(along);
      let (inside, piece) = locate(&u, &points, curve, p, &tmp);
      if_by(inside, || {
        let (a, b) = (piece.start(), piece.end());
        found.store(val(true));
        t.store((p - a).into_f32() / (b - a).into_f32());
        let (ca, cb) = (curve.canonical(a), curve.canonical(b));
        let point = |c: Node<u32>| (axis_v.select(c, coord), axis_v.select(coord, c));
        let ((au, av), (bu, bv)) = (point(ca), point(cb));
        ends.store(uvec4(au, av, bu, bv));
        cx.do_break();
      });
    });

    if_by(found.load(), || {
      if_by(round.greater_equal_than(u.rounds), || {
        set_overflow(&counters, OVERFLOW_ROUNDS);
      })
      .else_by(|| {
        let next_start = round_start(&counters, round + val(1));
        let local = counters
          .index(val(COUNTER_ROUNDS) + round + val(1))
          .atomic_add(val(2));
        let r = next_start + local;
        if_by((r + val(2)).greater_than(u.request_capacity), || {
          set_overflow(&counters, OVERFLOW_REQUESTS);
        })
        .else_by(|| {
          let ends = ends.load();
          requests
            .index(r)
            .store(uvec4(item_index, ends.x(), ends.y(), val(0)));
          requests
            .index(r + val(1))
            .store(uvec4(item_index, ends.z(), ends.w(), val(0)));
          let child = u.vertex_capacity + r;
          let record = uvec4(
            t.load().bitcast::<u32>(),
            child,
            child + val(1),
            val(RECORD_INTERPOLATE),
          );
          records.index(q).store(record);
          let x = (local + val(2) + val(VERTEX_WORKGROUP - 1)) / val(VERTEX_WORKGROUP);
          args.index(val(0)).atomic_max(x);
          args.index(val(1)).atomic_store(val(1));
          args.index(val(2)).atomic_store(val(1));
          do_return();
        });
      });
    });

    // Exact: patch boundary points on the canonical boundary curve, see the CPU `exact`.
    records.index(q).store(zeroed_val());
    let boundary_u = pu.equals(val(0)).or(pu.equals(val(PARAM_ONE)));
    let boundary_v = pv.equals(val(0)).or(pv.equals(val(PARAM_ONE)));
    let position = zeroed_val::<Vec3<f32>>().make_local_var();
    if_by(boundary_u.or(boundary_v), || {
      let axis_v = boundary_u.not();
      let coord = boundary_u.select(pu, pv);
      let along = boundary_u.select(pv, pu);
      let curve = load_line_curve(&cp, n, m, axis_v, coord, &points, &tmp);
      let t = param_to_f32(curve.canonical(along));
      position.store(to_euclid(de_casteljau_in_place(&points, curve.degree, t)));
    })
    .else_by(|| {
      let h = surface_eval(&cp, n, m, param_to_f32(pu), param_to_f32(pv));
      position.store(to_euclid(h));
    });
    let p = position.load();
    values.index(q).store(fvec4(p.x(), p.y(), p.z(), val(1.)));
  }
}

/// V backward round: interpolated values from the next round's values.
struct BackwardPass;
impl TessPass for BackwardPass {
  const WORKGROUP: (u32, u32, u32) = (VERTEX_WORKGROUP, 1, 1);
  type Bindings<K: BindKind> = (
    (UniformOut<K>, StepOut<K>),
    (
      Out<K, Rw<TessLeafDraw>>,
      Out<K, Rw<Vec4<u32>>>,
      Out<K, Rw<u32>>,
      Out<K, Atomics>,
    ),
    (Out<K, Rw<Vec4<u32>>>, Out<K, Rw<Vec4<f32>>>),
  );
  fn declare<B: Binder>(cx: &PassCtx, b: &mut B) -> Self::Bindings<B::Kind> {
    (
      (b.bind(&cx.frame.uniform), b.bind(cx.step())),
      (
        b.bind(&cx.frame.output.leaves),
        b.bind(&cx.frame.leaf_keys),
        b.bind(&cx.frame.output.bucket_leaves),
        b.bind(&cx.frame.counters),
      ),
      (b.bind(&cx.frame.records), b.bind(&cx.frame.values)),
    )
  }
  fn build(builder: &ShaderComputePipelineBuilder, b: Self::Bindings<ShaderKind>) {
    let ((u, step), (leaves, leaf_keys, bucket_leaves, counters), (records, values)) = b;
    let u = u.load().expand();
    let round = step.load().expand().step;
    let id = builder.global_invocation_id().x();
    let index = val(0u32).make_local_var();
    if_by(round.equals(val(0)), || {
      let v = leaf_vertex(&u, id, &counters, &bucket_leaves, &leaves, &leaf_keys);
      if_by(v.boundary.not(), do_return);
      index.store(v.vertex);
    })
    .else_by(|| {
      let r = round_request(&u, id, round, &counters);
      index.store(u.vertex_capacity + r);
    });
    let q = index.load();
    let record = records.index(q).load();
    if_by(record.w().equals(val(RECORD_INTERPOLATE)), || {
      let a = values.index(record.y()).load();
      let b = values.index(record.z()).load();
      values
        .index(q)
        .store(blend4(a, b, record.x().bitcast::<f32>()));
    });
  }
}

/// Final vertex pass: interior positions on the surface, boundary positions from the rounds,
/// normals from the surface partials of all vertices.
struct VertexPass;
impl TessPass for VertexPass {
  const WORKGROUP: (u32, u32, u32) = (VERTEX_WORKGROUP, 1, 1);
  type Bindings<K: BindKind> = (
    (UniformOut<K>, Out<K, Ro<TessItem>>),
    (
      Out<K, Ro<Vec4<f32>>>,
      Out<K, Ro<Vec2<u32>>>,
      Out<K, Ro<Mat4<f32>>>,
    ),
    (
      Out<K, Rw<TessLeafDraw>>,
      Out<K, Rw<Vec4<u32>>>,
      Out<K, Rw<u32>>,
      Out<K, Atomics>,
    ),
    (Out<K, Rw<Vec4<f32>>>, Out<K, Rw<TessVertex>>),
  );
  fn declare<B: Binder>(cx: &PassCtx, b: &mut B) -> Self::Bindings<B::Kind> {
    (
      (b.bind(&cx.frame.uniform), b.bind(&cx.data.items)),
      (
        b.bind(&cx.data.cp_pool),
        b.bind(&cx.data.cp_sources),
        b.bind(&cx.data.matrices),
      ),
      (
        b.bind(&cx.frame.output.leaves),
        b.bind(&cx.frame.leaf_keys),
        b.bind(&cx.frame.output.bucket_leaves),
        b.bind(&cx.frame.counters),
      ),
      (b.bind(&cx.frame.values), b.bind(&cx.frame.output.vertices)),
    )
  }
  fn build(builder: &ShaderComputePipelineBuilder, b: Self::Bindings<ShaderKind>) {
    let ((u, items), (pool, sources, matrices), leaf_bindings, (values, vertices)) = b;
    let (leaves, leaf_keys, bucket_leaves, counters) = leaf_bindings;
    let u = u.load().expand();
    let id = builder.global_invocation_id().x();
    let v = leaf_vertex(&u, id, &counters, &bucket_leaves, &leaves, &leaf_keys);
    let item = items.index(v.key.item).load().expand();
    // The partials come from the face origin relative control points (small values, the
    // camera space ones lose the precision of small patches far from the camera) mapped by
    // the instance matrix.
    let pool_offset = sources.index(item.camera_cp_offset).load().x();
    let row = item.degree_u + val(1);
    let cp = |i: Node<u32>, j: Node<u32>| pool.index(pool_offset + j * row + i).load();
    let matrix = matrices.index(item.instance).load();
    let partials = |pu: Node<f32>, pv: Node<f32>| {
      let (p, du, dv) = surface_eval_with_partials(&cp, item.degree_u, item.degree_v, pu, pv);
      let map = |x: Node<Vec3<f32>>, w: f32| (matrix * fvec4(x.x(), x.y(), x.z(), val(w))).xyz();
      (map(p, 1.), map(du, 0.), map(dv, 0.))
    };
    let (pu, pv) = (param_to_f32(v.u()), param_to_f32(v.v()));
    let (p, du, dv) = partials(pu, pv);
    let position = v.boundary.select(values.index(v.vertex).load().xyz(), p);
    // At a pole (a numerically zero partial) the normal is taken slightly inside the leaf.
    let (lu, lv) = (du.length(), dv.length());
    let degenerate = lu.min(lv).less_equal_than(lu.max(lv) * val(1e-5));
    let normal = du.cross(dv).make_local_var();
    if_by(degenerate, || {
      let (cu, cv) = v.center();
      let nudge = val(1. / 64.);
      let (_, du, dv) = partials(pu + (cu - pu) * nudge, pv + (cv - pv) * nudge);
      normal.store(du.cross(dv));
    });
    let normal = normal.load();
    let length = normal.length();
    let normal = length.greater_than(val(0.)).select(normal / length, normal);
    let flip = (item.flags & val(ITEM_FLAG_FLIP_NORMAL)).not_equals(val(0));
    let normal = flip.select(-normal, normal);
    let vertex = ENode::<TessVertex> {
      position: fvec4(position.x(), position.y(), position.z(), val(1.)),
      normal: fvec4(normal.x(), normal.y(), normal.z(), val(0.)),
    }
    .construct();
    vertices.index(v.vertex).store(vertex);
  }
}
