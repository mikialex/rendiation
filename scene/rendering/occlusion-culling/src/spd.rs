use rendiation_shader_api::*;

use crate::*;

const TILE_SIZE: u32 = 64;
const INTERMEDIATE_SIZE: usize = 16;

type IntermediateBuffer<T> = [[T; INTERMEDIATE_SIZE]; INTERMEDIATE_SIZE];

#[derive(Clone, Copy, ShaderStruct)]
pub struct Ctx {
  pub coord: Vec2<u32>,
  pub group_id: Vec2<u32>,
  pub local_invocation_index: u32,
  pub mip_level_count: u32,
}

pub trait QuadReducer<T>: Copy + Clone + 'static {
  fn reduce(&self, v: LocalVarNode<[T; 4]>) -> Node<T>;
}

#[derive(Clone, Copy)]
pub struct MinReducer;
impl<T: PrimitiveShaderNodeType> QuadReducer<T> for MinReducer {
  fn reduce(&self, v: LocalVarNode<[T; 4]>) -> Node<T> {
    let v1 = v.index(0).load();
    let v2 = v.index(1).load();
    let v3 = v.index(2).load();
    let v4 = v.index(3).load();
    v1.min(v2).min(v3).min(v4)
  }
}

#[derive(Clone, Copy)]
pub struct MaxReducer;
impl<T: PrimitiveShaderNodeType> QuadReducer<T> for MaxReducer {
  fn reduce(&self, v: LocalVarNode<[T; 4]>) -> Node<T> {
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

impl<T> SourceImageLoader<T::Output> for HandleNode<T>
where
  T: ShaderDirectLoad + SingleLayerTarget + SingleSampleTarget,
  Node<T::LoadInput>: From<Node<Vec2<u32>>>,
{
  fn load(&self, coord: Node<Vec2<u32>>) -> Node<T::Output> {
    self.load_texel(coord.into(), val(0))
  }
}

impl<T> SourceImageWriter<T::Output> for HandleNode<T>
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
    .insert_bits(a, val(0), val(2));

  (x, y).into()
}

pub struct SpdImageDownSampler<S, R, N> {
  loader: S,
  reducer: R,
  quad: LocalVarNode<[N; 4]>,
}

impl<S, R, N> SpdImageDownSampler<S, R, N>
where
  S: SourceImageLoader<N>,
  R: QuadReducer<N>,
  N: ShaderSizedValueNodeType,
{
  pub fn new(loader: S, reducer: R) -> Self {
    Self {
      loader,
      reducer,
      quad: zeroed_val().make_local_var(),
    }
  }

  pub fn down_sample(&self, tex: Node<Vec2<u32>>) -> Node<N> {
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
    self.reducer.reduce(self.quad)
  }
}

pub struct SpdIntermediateDownSampler<T, R> {
  intermediate: WorkGroupSharedNode<IntermediateBuffer<T>>,
  reducer: R,
  quad: LocalVarNode<[T; 4]>,
}

impl<T, R> SpdIntermediateDownSampler<T, R>
where
  T: ShaderSizedValueNodeType,
  R: QuadReducer<T>,
{
  pub fn new(intermediate: WorkGroupSharedNode<IntermediateBuffer<T>>, reducer: R) -> Self {
    Self {
      intermediate,
      reducer,
      quad: zeroed_val().make_local_var(),
    }
  }

  pub fn down_sample(&self, coords: [impl Into<Node<Vec2<u32>>>; 4]) -> Node<T> {
    for (i, tex) in coords.into_iter().enumerate() {
      let tex = tex.into();
      let v = self.intermediate.index(tex.x()).index(tex.y()).load();
      self.quad.index(val(i as u32)).store(v);
    }
    self.reducer.reduce(self.quad)
  }
}

pub fn down_sample_mips_0_and_1<S, N, R, T>(
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

  let sub_tile_reduced: LocalVarNode<[N; 4]> = zeroed_val().make_local_var();

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
