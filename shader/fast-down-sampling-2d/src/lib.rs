use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod reducer;
pub use reducer::*;

mod entry;
pub use entry::*;

mod io;
pub use io::*;

pub const MAX_INPUT_SIZE: u32 = 2_u32.pow(12); // 4096

pub fn fast_down_sampling<V>(
  reducer: &dyn QuadReducer<V>,
  io: &dyn FastDownSamplingIO<V>,
  pass: &mut GPUComputePass,
  device: &GPUDevice,
) where
  V: ShaderSizedValueNodeType,
{
  let (width, height) = io.root_size();
  let mip_level_count = io.mip_level_count();
  // check input
  assert!(width <= MAX_INPUT_SIZE);
  assert!(height <= MAX_INPUT_SIZE);
  assert_eq!(
    mip_level_count,
    MipLevelCount::BySize.get_level_count_wgpu(Size::from_u32_pair_min_one((width, height)))
  );

  // we can not read from the texture meta in shader, because we want
  // the mip_count for full texture size
  let mip_count_buffer = create_uniform(Vec4::new(mip_level_count, 0, 0, 0), device);

  // first pass
  {
    let hasher = shader_hasher_from_marker_ty!(SPDxFirstPass);
    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut ctx| {
      ctx.config_work_group_size(16 * 16);
      let group_id = ctx.workgroup_id().xy();
      let local_id = ctx.local_invocation_index();
      let coord = remap_for_wave_reduction_fn(local_id % val(64));

      // map to 16 * 16 grid
      let x = coord.x() + ((local_id >> val(6)) % val(2)) * val(8);
      let y = coord.y() + (local_id >> val(7)) * val(8);
      let coord = (x, y).into();

      let mip_level_count = ctx.bind_by(&mip_count_buffer).load().x();

      let stage_one_io = io.bind_first_stage_shader(&mut ctx);

      let shared = SharedMemoryDownSampler::new(&ctx);

      let sample_ctx = ENode::<SampleCtx> {
        coord,
        group_id,
        local_invocation_index: local_id,
        mip_level_count,
      };

      down_sample_mips_0_and_1(
        stage_one_io.get_root_loader_with_possible_write().as_ref(),
        &shared,
        stage_one_io.get_1_6_level_writer(1).as_ref(),
        stage_one_io.get_1_6_level_writer(2).as_ref(),
        sample_ctx,
        reducer,
      );

      down_sample_next_four(
        &shared,
        stage_one_io.get_1_6_level_writer(3).as_ref(),
        stage_one_io.get_1_6_level_writer(4).as_ref(),
        stage_one_io.get_1_6_level_writer(5).as_ref(),
        stage_one_io.get_1_6_level_writer(6).as_ref(),
        sample_ctx,
        val(2),
        reducer,
      );

      ctx
    });

    BindingBuilder::default()
      .with_bind(&mip_count_buffer)
      .with_fn(|b| {
        io.bind_first_stage_pass(b);
      })
      .setup_compute_pass(pass, device, &pipeline);

    let x_workgroup_required = width.div_ceil(64);
    let y_workgroup_required = height.div_ceil(64);
    pass.dispatch_workgroups(x_workgroup_required, y_workgroup_required, 1);
  }

  if mip_level_count < 7 {
    return;
  }

  // second pass
  {
    let hasher = shader_hasher_from_marker_ty!(SPDxSecondPass);
    let pipeline = device.get_or_cache_create_compute_pipeline_by(hasher, |mut ctx| {
      ctx.config_work_group_size(16 * 16);
      let group_id = ctx.workgroup_id().xy();
      let local_id = ctx.local_invocation_index();

      let coord = remap_for_wave_reduction_fn(local_id % val(64));

      // map to 16 * 16 grid
      let x = coord.x() + ((local_id >> val(6)) % val(2)) * val(8);
      let y = coord.y() + (local_id >> val(7)) * val(8);
      let coord = (x, y).into();

      let mip_level_count = ctx.bind_by(&mip_count_buffer).load().x();

      let stage_two_io = io.bind_second_stage_shader(&mut ctx);

      let shared_sampler = SharedMemoryDownSampler::new(&ctx);

      let sample_ctx = ENode::<SampleCtx> {
        coord,
        group_id,
        local_invocation_index: local_id,
        mip_level_count,
      };

      down_sample_mips_6_and_7(
        stage_two_io.get_level_6_loader().as_ref(),
        &shared_sampler,
        stage_two_io.get_7_12_level_writer(7).as_ref(),
        stage_two_io.get_7_12_level_writer(8).as_ref(),
        sample_ctx,
        reducer,
      );

      down_sample_next_four(
        &shared_sampler,
        stage_two_io.get_7_12_level_writer(9).as_ref(),
        stage_two_io.get_7_12_level_writer(10).as_ref(),
        stage_two_io.get_7_12_level_writer(11).as_ref(),
        stage_two_io.get_7_12_level_writer(12).as_ref(),
        sample_ctx,
        val(8),
        reducer,
      );

      ctx
    });

    BindingBuilder::default()
      .with_bind(&mip_count_buffer)
      .with_fn(|bb| {
        io.bind_second_stage_pass(bb);
      })
      .setup_compute_pass(pass, device, &pipeline);

    pass.dispatch_workgroups(1, 1, 1);
  }
}

const TILE_SIZE: u32 = 64;
const SHARED_SIZE: usize = 16;
type SharedMemory<T> = [[T; SHARED_SIZE]; SHARED_SIZE];

#[derive(Clone, Copy, ShaderStruct)]
struct SampleCtx {
  pub coord: Vec2<u32>,
  pub group_id: Vec2<u32>,
  pub local_invocation_index: u32,
  pub mip_level_count: u32,
}

/// remap to 8 x 8 grid point
#[shader_fn]
fn remap_for_wave_reduction(a: Node<u32>) -> Node<Vec2<u32>> {
  let x = a
    .extract_bits(val(2), val(3))
    .insert_bits(a, val(0), val(1));
  let n = a.extract_bits(val(1), val(2));
  let y = a
    .extract_bits(val(3), val(3))
    .insert_bits(n, val(0), val(2));

  (x, y).into()
}

struct SharedMemoryDownSampler<T> {
  shared: ShaderPtrOf<SharedMemory<T>>,
}

impl<T> SharedMemoryDownSampler<T>
where
  T: ShaderSizedValueNodeType,
{
  fn new(ctx: &ShaderComputePipelineBuilder) -> Self {
    Self {
      shared: ctx.define_workgroup_shared_var::<SharedMemory<T>>(),
    }
  }

  fn store(&self, coord: Node<Vec2<u32>>, value: Node<T>) {
    self.shared.index(coord.x()).index(coord.y()).store(value);
  }

  fn down_sample(
    &self,
    coords: [impl Into<Node<Vec2<u32>>>; 4],
    reducer: &dyn QuadReducer<T>,
  ) -> Node<T> {
    let loads = coords.map(|coord| {
      let coord = coord.into();
      self.shared.index(coord.x()).index(coord.y()).load()
    });
    reducer.reduce(loads)
  }
}

fn down_sample_mips_0_and_1<N>(
  image_sampler: &dyn SourceImageLoader<N>,
  shared_sampler: &SharedMemoryDownSampler<N>,
  l_1: &dyn SourceImageWriter<N>,
  l_2: &dyn SourceImageWriter<N>,
  sample_ctx: ENode<SampleCtx>,
  reducer: &dyn QuadReducer<N>,
) where
  N: ShaderSizedValueNodeType,
{
  let ENode::<SampleCtx> {
    coord,
    group_id,
    local_invocation_index,
    mip_level_count,
  } = sample_ctx;

  let sub_tile_reduced = zeroed_val::<[N; 4]>().make_local_var();

  for (i, o) in [vec2(0, 0), vec2(16, 0), vec2(0, 16), vec2(16, 16)]
    .into_iter()
    .enumerate()
  {
    let pix = group_id * val(TILE_SIZE / 2) + coord + val(o);
    let tex = pix * val(2);
    let reduced = image_sampler.down_sample_quad(tex, reducer);
    sub_tile_reduced.index(val(i as u32)).store(reduced);
    l_1.write(pix, reduced);
  }

  if_by(mip_level_count.less_equal_than(val(1)), do_return);

  4.into_shader_iter().for_each(|i, _| {
    shared_sampler.store(coord, sub_tile_reduced.index(i).load());

    workgroup_barrier();

    if_by(local_invocation_index.less_equal_than(val(64)), || {
      let scaled = coord * val(2);
      let reduced = shared_sampler.down_sample(
        [
          scaled + val(vec2(0, 0)),
          scaled + val(vec2(1, 0)),
          scaled + val(vec2(0, 1)),
          scaled + val(vec2(1, 1)),
        ],
        reducer,
      );
      let xy: Node<Vec2<u32>> = (i % val(2), i / val(2)).into();
      let pix = group_id * val(TILE_SIZE / 4) + xy * val(8) + coord;
      l_2.write(pix, reduced);
      sub_tile_reduced.index(i).store(reduced);
    });

    // is this required?
    workgroup_barrier();
  });

  if_by(local_invocation_index.less_than(val(64)), || {
    for (i, o) in [vec2(0, 0), vec2(8, 0), vec2(0, 8), vec2(8, 8)]
      .into_iter()
      .enumerate()
    {
      let coord = coord + val(o);
      shared_sampler.store(coord, sub_tile_reduced.index(i as u32).load());
    }
  });
}

fn down_sample_mips_6_and_7<N>(
  image_sampler: &dyn SourceImageLoader<N>,
  shared_sampler: &SharedMemoryDownSampler<N>,
  l_7: &dyn SourceImageWriter<N>,
  l_8: &dyn SourceImageWriter<N>,
  sample_ctx: ENode<SampleCtx>,
  reducer: &dyn QuadReducer<N>,
) where
  N: ShaderSizedValueNodeType,
{
  let ENode::<SampleCtx> {
    coord,
    mip_level_count,
    ..
  } = sample_ctx;

  let reduced = [vec2(0, 0), vec2(1, 0), vec2(0, 1), vec2(1, 1)].map(|offset| {
    let pix = coord * val(2) + val(offset);
    let coord = pix * val(2);
    let reduced = image_sampler.down_sample_quad(coord, reducer);
    l_7.write(pix, reduced);
    reduced
  });

  if_by(mip_level_count.less_equal_than(val(7)), do_return);

  let l_8_local = reducer.reduce(reduced);
  l_8.write(coord, l_8_local);
  shared_sampler.store(coord, l_8_local);
}

fn down_sample_next_four<N>(
  sampler: &SharedMemoryDownSampler<N>,
  l_3: &dyn SourceImageWriter<N>,
  l_4: &dyn SourceImageWriter<N>,
  l_5: &dyn SourceImageWriter<N>,
  l_6: &dyn SourceImageWriter<N>,
  sample_ctx: ENode<SampleCtx>,
  base_mip: Node<u32>,
  reducer: &dyn QuadReducer<N>,
) where
  N: ShaderSizedValueNodeType,
{
  let ENode::<SampleCtx> {
    coord,
    group_id,
    local_invocation_index,
    mip_level_count,
  } = sample_ctx;

  if_by(mip_level_count.less_equal_than(base_mip), do_return);
  workgroup_barrier();

  if_by(local_invocation_index.less_than(val(TILE_SIZE)), || {
    let scaled = coord * val(2);
    let reduced = sampler.down_sample(
      [
        scaled + val(vec2(0, 0)),
        scaled + val(vec2(1, 0)),
        scaled + val(vec2(0, 1)),
        scaled + val(vec2(1, 1)),
      ],
      reducer,
    );

    let x = coord.x() * val(2) + coord.y() % val(2);
    let y = coord.y() * val(2);
    sampler.shared.index(x).index(y).store(reduced);
    l_3.write(group_id * val(TILE_SIZE / 8) + coord, reduced);
  });
  if_by(
    mip_level_count.less_equal_than(base_mip + val(1)),
    do_return,
  );
  workgroup_barrier();

  if_by(
    local_invocation_index.less_than(val(TILE_SIZE / 16)),
    || {
      let scaled = coord * val(4);
      let reduced = sampler.down_sample(
        [
          scaled + val(vec2(0, 0)),
          scaled + val(vec2(2, 0)),
          scaled + val(vec2(0, 2)),
          scaled + val(vec2(1, 2)),
        ],
        reducer,
      );

      let x = coord.x() * val(4) + coord.y(); // checked, not required % val(4)
      let y = coord.y() * val(4);
      sampler.shared.index(x).index(y).store(reduced);
      l_4.write(group_id * val(TILE_SIZE / 16) + coord, reduced);
    },
  );
  if_by(
    mip_level_count.less_equal_than(base_mip + val(2)),
    do_return,
  );
  workgroup_barrier();

  if_by(
    local_invocation_index.less_than(val(TILE_SIZE / 16)),
    || {
      let scaled = coord * val(8);
      let reduced = sampler.down_sample(
        [
          scaled + (coord.y() * val(2), val(0)).into(),
          scaled + (coord.y() * val(2) + val(4), val(0)).into(),
          scaled + (coord.y() * val(2) + val(1), val(4)).into(),
          scaled + (coord.y() * val(2) + val(5), val(4)).into(),
        ],
        reducer,
      );

      let x = coord.x() + coord.y() * val(2);
      sampler.shared.index(x).index(0).store(reduced);
      l_5.write(group_id * val(TILE_SIZE / 32) + coord, reduced);
    },
  );
  if_by(
    mip_level_count.less_equal_than(base_mip + val(3)),
    do_return,
  );
  workgroup_barrier();

  if_by(
    local_invocation_index.less_than(val(TILE_SIZE / 64)),
    || {
      let reduced = sampler.down_sample([vec2(0, 0), vec2(1, 0), vec2(2, 0), vec2(3, 0)], reducer);
      l_6.write(group_id, reduced);
    },
  );
}
