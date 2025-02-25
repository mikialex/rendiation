use rendiation_shader_api::*;

use crate::*;

/// the target is a h depth texture, user should guarantee it has correct size and format.
///
/// todo, error handling for too large input
pub fn compute_hierarchy_depth_from_multi_sample_depth_texture(
  ms_depth: &GPUMultiSample2DDepthTextureView,
  target: &GPU2DTexture,
  pass: &mut GPUComputePass,
  device: &GPUDevice,
) {
  let x = target.desc.size.width.div_ceil(64);
  let y = target.desc.size.height.div_ceil(64);
  let mip_level_count = target.desc.mip_level_count;

  // level that exceeds will be clamped to max level
  let mips: [GPU2DTextureView; 13] = std::array::from_fn(|index| {
    target
      .create_view(TextureViewDescriptor {
        base_mip_level: (index as u32).clamp(0, mip_level_count - 1),
        mip_level_count: Some(1),
        base_array_layer: 0,
        ..Default::default()
      })
      .try_into()
      .unwrap()
  });

  let level_0 = mips[0]
    .clone()
    .into_storage_texture_view_writeonly()
    .unwrap();
  let mip_count_buffer = create_uniform(Vec4::new(mip_level_count, 0, 0, 0), device);
  let level_1_6: [StorageTextureViewWriteOnly<GPU2DTextureView>; 6] = std::array::from_fn(|i| {
    mips[i + 1]
      .clone()
      .into_storage_texture_view_writeonly()
      .unwrap()
  });

  let hasher = shader_hasher_from_marker_ty!(SPDxFirstPass);
  let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |mut ctx| {
    ctx.config_work_group_size(256);
    let shared = ctx.define_workgroup_shared_var::<IntermediateBuffer<f32>>();
    let group_id = ctx.workgroup_id().xy();
    let local_id = ctx.local_invocation_index();
    let coord = remap_for_wave_reduction_fn(local_id % val(64));

    // map to 16 * 16 grid
    let x = coord.x() + ((local_id >> val(6)) % val(2)) * val(8);
    let y = coord.y() + (local_id >> val(7)) * val(8);
    let coord = (x, y).into();

    let ms_depth = ctx.bind_by(&ms_depth);
    let mip_0 = ctx.bind_by(&level_0);
    let mip_level_count = ctx.bind_by(&mip_count_buffer).load().x();

    let scale =
      ms_depth.texture_dimension_2d(None).into_f32() / mip_0.texture_dimension_2d(None).into_f32();

    let image_loader = MSDepthLoader {
      ms_depth,
      mip_0,
      scale,
    };

    let image_sampler = SpdImageDownSampler::new(image_loader, MaxReducer);
    let intermediate_sampler = SpdIntermediateDownSampler::new(shared, MaxReducer);

    let sample_ctx = ENode::<Ctx> {
      coord,
      group_id,
      local_invocation_index: local_id,
      mip_level_count,
    }
    .construct();

    down_sample_mips_0_and_1::<_, _, _, SplatWriter>(
      &image_sampler,
      &intermediate_sampler,
      ctx.bind_by(&level_1_6[0]).into(),
      ctx.bind_by(&level_1_6[1]).into(),
      sample_ctx,
    );

    down_sample_next_four::<_, _, SplatWriter>(
      &intermediate_sampler,
      ctx.bind_by(&level_1_6[2]).into(),
      ctx.bind_by(&level_1_6[3]).into(),
      ctx.bind_by(&level_1_6[4]).into(),
      ctx.bind_by(&level_1_6[5]).into(),
      sample_ctx,
      val(2),
    );

    ctx
  });

  let mut bb = BindingBuilder::default()
    .with_bind(ms_depth)
    .with_bind(&level_0);
  for v in level_1_6.iter() {
    bb.bind(v);
  }
  bb.setup_compute_pass(pass, device, &pipeline);
  pass.dispatch_workgroups(x, y, 1);

  if mip_level_count < 7 {
    return;
  }

  let l_6 = mips[6].clone();
  let l_7_12: [StorageTextureViewWriteOnly<GPU2DTextureView>; 6] = std::array::from_fn(|i| {
    mips[i + 7]
      .clone()
      .into_storage_texture_view_writeonly()
      .unwrap()
  });

  let hasher = shader_hasher_from_marker_ty!(SPDxSecondPass);
  let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |mut ctx| {
    ctx.config_work_group_size(256);
    let shared = ctx.define_workgroup_shared_var::<IntermediateBuffer<f32>>();
    let group_id = ctx.workgroup_id().xy();
    let local_id = ctx.local_invocation_index();

    let coord = remap_for_wave_reduction_fn(local_id % val(64));

    // map to 16 * 16 grid
    let x = coord.x() + ((local_id >> val(6)) % val(2)) * val(8);
    let y = coord.y() + (local_id >> val(7)) * val(8);
    let coord = (x, y).into();

    let mip_level_count = ctx.bind_by(&mip_count_buffer).load().x();

    let image_sampler = SpdImageDownSampler::new(
      LoadFirstChannel {
        source: ctx.bind_by(&l_6),
      },
      MaxReducer,
    );
    let intermediate_sampler = SpdIntermediateDownSampler::new(shared, MaxReducer);

    let sample_ctx = ENode::<Ctx> {
      coord,
      group_id,
      local_invocation_index: local_id,
      mip_level_count,
    }
    .construct();

    down_sample_mips_6_and_7::<_, _, _, SplatWriter>(
      &image_sampler,
      &intermediate_sampler,
      ctx.bind_by(&l_7_12[0]).into(),
      ctx.bind_by(&l_7_12[1]).into(),
      sample_ctx,
    );

    down_sample_next_four::<_, _, SplatWriter>(
      &intermediate_sampler,
      ctx.bind_by(&l_7_12[2]).into(),
      ctx.bind_by(&l_7_12[3]).into(),
      ctx.bind_by(&l_7_12[4]).into(),
      ctx.bind_by(&l_7_12[5]).into(),
      sample_ctx,
      val(8),
    );

    ctx
  });

  let mut bb = BindingBuilder::default()
    .with_bind(&mip_count_buffer)
    .with_bind(&l_6);
  for v in l_7_12.iter() {
    bb.bind(v);
  }
  bb.setup_compute_pass(pass, device, &pipeline);
  pass.dispatch_workgroups(1, 1, 1);
}

struct MSDepthLoader {
  mip_0: BindingNode<ShaderStorageTextureW2D>,
  ms_depth: BindingNode<ShaderMultiSampleDepthTexture2D>,
  scale: Node<Vec2<f32>>,
}

impl SourceImageLoader<f32> for MSDepthLoader {
  fn load(&self, coord: Node<Vec2<u32>>) -> Node<f32> {
    let depth_coord = coord.into_f32() * self.scale;
    let depth_coord = depth_coord.round().into_u32();

    let d1 = self
      .ms_depth
      .load_texel_multi_sample_index(depth_coord, val(0));
    let d2 = self
      .ms_depth
      .load_texel_multi_sample_index(depth_coord, val(1));
    let d3 = self
      .ms_depth
      .load_texel_multi_sample_index(depth_coord, val(2));
    let d4 = self
      .ms_depth
      .load_texel_multi_sample_index(depth_coord, val(3));

    let v = (d1 + d2 + d3 + d4) / val(4.); // todo fix me, this is wrong!
    self.mip_0.write_texel(coord, v.splat());
    v
  }
}

struct LoadFirstChannel {
  source: BindingNode<ShaderTexture2D>,
}
impl SourceImageLoader<f32> for LoadFirstChannel {
  fn load(&self, coord: Node<Vec2<u32>>) -> Node<f32> {
    self.source.load(coord).x()
  }
}

struct SplatWriter {
  target: BindingNode<ShaderStorageTextureW2D>,
}
impl From<BindingNode<ShaderStorageTextureW2D>> for SplatWriter {
  fn from(target: BindingNode<ShaderStorageTextureW2D>) -> Self {
    Self { target }
  }
}
impl SourceImageWriter<f32> for SplatWriter {
  fn write(&self, coord: Node<Vec2<u32>>, value: Node<f32>) {
    self.target.write(coord, value.splat());
  }
}

const TILE_SIZE: u32 = 64;
const INTERMEDIATE_SIZE: usize = 16;

type IntermediateBuffer<T> = [[T; INTERMEDIATE_SIZE]; INTERMEDIATE_SIZE];

#[derive(Clone, Copy, ShaderStruct)]
struct Ctx {
  pub coord: Vec2<u32>,
  pub group_id: Vec2<u32>,
  pub local_invocation_index: u32,
  pub mip_level_count: u32,
}

pub trait QuadReducer<T>: Copy + Clone + 'static {
  fn reduce(&self, v: &ShaderPtrOf<[T; 4]>) -> Node<T>;
}

#[derive(Clone, Copy)]
pub struct MaxReducer;
impl<T: PrimitiveShaderNodeType> QuadReducer<T> for MaxReducer {
  fn reduce(&self, v: &ShaderPtrOf<[T; 4]>) -> Node<T> {
    let v1 = v.index(0).load();
    let v2 = v.index(1).load();
    let v3 = v.index(2).load();
    let v4 = v.index(3).load();
    v1.max(v2).max(v3).max(v4)
  }
}

pub trait SourceImageLoader<V: ShaderNodeType> {
  fn load(&self, coord: Node<Vec2<u32>>) -> Node<V>;
}

pub trait SourceImageWriter<V: ShaderNodeType> {
  fn write(&self, coord: Node<Vec2<u32>>, value: Node<V>);
}

impl<T> SourceImageLoader<T::Output> for BindingNode<T>
where
  T: ShaderDirectLoad + SingleLayerTarget + SingleSampleTarget,
  Node<T::LoadInput>: From<Node<Vec2<u32>>>,
{
  fn load(&self, coord: Node<Vec2<u32>>) -> Node<T::Output> {
    self.load_texel(coord.into(), val(0))
  }
}

impl<T> SourceImageWriter<T::Output> for BindingNode<T>
where
  T: ShaderStorageTextureLike + ShaderDirectLoad + SingleLayerTarget,
  Node<T::LoadInput>: From<Node<Vec2<u32>>>,
{
  fn write(&self, coord: Node<Vec2<u32>>, value: Node<T::Output>) {
    self.write_texel(coord.into(), value);
  }
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

struct SpdImageDownSampler<S, R, N> {
  loader: S,
  reducer: R,
  quad: ShaderPtrOf<[N; 4]>,
}

impl<S, R, N> SpdImageDownSampler<S, R, N>
where
  S: SourceImageLoader<N>,
  R: QuadReducer<N>,
  N: ShaderSizedValueNodeType,
{
  fn new(loader: S, reducer: R) -> Self {
    Self {
      loader,
      reducer,
      quad: zeroed_val::<[N; 4]>().make_local_var(),
    }
  }

  fn down_sample(&self, tex: Node<Vec2<u32>>) -> Node<N> {
    let offsets = [
      vec2(0_u32, 0_u32),
      vec2(0_u32, 1_u32),
      vec2(1_u32, 0_u32),
      vec2(1_u32, 1_u32),
    ];
    for (i, o) in offsets.into_iter().enumerate() {
      let v = self.loader.load(tex + val(o));
      self.quad.index(val(i as u32)).store(v);
    }
    // todo, boundary check?
    self.reducer.reduce(&self.quad)
  }
}

struct SpdIntermediateDownSampler<T, R> {
  intermediate: ShaderPtrOf<IntermediateBuffer<T>>,
  reducer: R,
  quad: ShaderPtrOf<[T; 4]>,
}

impl<T, R> SpdIntermediateDownSampler<T, R>
where
  T: ShaderSizedValueNodeType,
  R: QuadReducer<T>,
{
  fn new(intermediate: ShaderPtrOf<IntermediateBuffer<T>>, reducer: R) -> Self {
    Self {
      intermediate,
      reducer,
      quad: zeroed_val::<[T; 4]>().make_local_var(),
    }
  }

  fn down_sample(&self, coords: [impl Into<Node<Vec2<u32>>>; 4]) -> Node<T> {
    for (i, tex) in coords.into_iter().enumerate() {
      let tex = tex.into();
      let v = self.intermediate.index(tex.x()).index(tex.y()).load();
      self.quad.index(val(i as u32)).store(v);
    }
    self.reducer.reduce(&self.quad)
  }
}

fn down_sample_mips_0_and_1<S, N, R, T>(
  image_sampler: &SpdImageDownSampler<S, R, N>,
  intermediate_sampler: &SpdIntermediateDownSampler<N, R>,
  l_1: T,
  l_2: T,
  sample_ctx: Node<Ctx>,
) where
  N: ShaderSizedValueNodeType,
  R: QuadReducer<N>,
  S: SourceImageLoader<N>,
  T: SourceImageWriter<N>,
{
  let ENode::<Ctx> {
    coord,
    group_id,
    local_invocation_index,
    mip_level_count,
  } = sample_ctx.expand();

  let sub_tile_reduced = zeroed_val::<[N; 4]>().make_local_var();

  for (i, o) in [vec2(0, 0), vec2(16, 0), vec2(0, 16), vec2(16, 16)]
    .into_iter()
    .enumerate()
  {
    let pix = group_id * val(TILE_SIZE / 2) + coord + val(o);
    let tex = pix * val(2);
    let reduced = image_sampler.down_sample(tex);
    sub_tile_reduced.index(val(i as u32)).store(reduced);
    l_1.write(pix, reduced);
  }

  if_by(mip_level_count.less_equal_than(val(1)), do_return);

  4_u32.into_shader_iter().for_each(|i, _| {
    intermediate_sampler
      .intermediate
      .index(coord.x())
      .index(coord.y())
      .store(sub_tile_reduced.index(i).load());

    workgroup_barrier();

    if_by(local_invocation_index.less_equal_than(val(64)), || {
      let scaled = coord * val(2);
      let reduced = intermediate_sampler.down_sample([
        scaled + val(vec2(0, 0)),
        scaled + val(vec2(1, 0)),
        scaled + val(vec2(0, 1)),
        scaled + val(vec2(1, 1)),
      ]);
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
      intermediate_sampler
        .intermediate
        .index(coord.x())
        .index(coord.y())
        .store(sub_tile_reduced.index(i as u32).load())
    }
  });
}

fn down_sample_mips_6_and_7<S, N, R, T>(
  image_sampler: &SpdImageDownSampler<S, R, N>,
  intermediate_sampler: &SpdIntermediateDownSampler<N, R>,
  l_7: T,
  l_8: T,
  sample_ctx: Node<Ctx>,
) where
  N: ShaderSizedValueNodeType,
  R: QuadReducer<N>,
  S: SourceImageLoader<N>,
  T: SourceImageWriter<N>,
{
  let ENode::<Ctx> {
    coord,
    mip_level_count,
    ..
  } = sample_ctx.expand();

  let l_7_local = zeroed_val::<[N; 4]>().make_local_var();

  for (i, o) in [vec2(0, 0), vec2(1, 0), vec2(0, 1), vec2(1, 1)]
    .into_iter()
    .enumerate()
  {
    let pix = coord * val(2) + val(o);
    let tex = pix * val(2);
    let reduced = image_sampler.down_sample(tex);
    l_7_local.index(val(i as u32)).store(reduced);
    l_7.write(pix, reduced);
  }

  if_by(mip_level_count.less_equal_than(val(7)), do_return);

  let l_8_local = intermediate_sampler.reducer.reduce(&l_7_local);
  l_8.write(coord, l_8_local);
  intermediate_sampler
    .intermediate
    .index(coord.x())
    .index(coord.y())
    .store(l_8_local);
}

fn down_sample_next_four<N, R, T>(
  sampler: &SpdIntermediateDownSampler<N, R>,
  l_3: T,
  l_4: T,
  l_5: T,
  l_6: T,
  sample_ctx: Node<Ctx>,
  base_mip: Node<u32>,
) where
  N: ShaderSizedValueNodeType,
  R: QuadReducer<N>,
  T: SourceImageWriter<N>,
{
  let ENode::<Ctx> {
    coord,
    group_id,
    local_invocation_index,
    mip_level_count,
  } = sample_ctx.expand();

  if_by(mip_level_count.less_equal_than(base_mip), do_return);
  workgroup_barrier();

  if_by(local_invocation_index.less_than(val(TILE_SIZE)), || {
    let scaled = coord * val(2);
    let reduced = sampler.down_sample([
      scaled + val(vec2(0, 0)),
      scaled + val(vec2(1, 0)),
      scaled + val(vec2(0, 1)),
      scaled + val(vec2(1, 1)),
    ]);

    let x = coord.x() * val(2) + coord.y() % val(2);
    let y = coord.y() * val(2);
    sampler.intermediate.index(x).index(y).store(reduced);
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
      let reduced = sampler.down_sample([
        scaled + val(vec2(0, 0)),
        scaled + val(vec2(2, 0)),
        scaled + val(vec2(0, 2)),
        scaled + val(vec2(1, 2)),
      ]);

      let x = coord.x() * val(4) + coord.y(); // checked, not required % val(4)
      let y = coord.y() * val(4);
      sampler.intermediate.index(x).index(y).store(reduced);
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
      let reduced = sampler.down_sample([
        scaled + (coord.y() * val(2), val(0)).into(),
        scaled + (coord.y() * val(2) + val(4), val(0)).into(),
        scaled + (coord.y() * val(2) + val(1), val(4)).into(),
        scaled + (coord.y() * val(2) + val(5), val(4)).into(),
      ]);

      let x = coord.x() + coord.y() * val(2);
      sampler.intermediate.index(x).index(0).store(reduced);
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
      let reduced = sampler.down_sample([vec2(0, 0), vec2(1, 0), vec2(2, 0), vec2(3, 0)]);
      l_6.write(group_id, reduced);
    },
  );
}
