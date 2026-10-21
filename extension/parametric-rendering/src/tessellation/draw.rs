//! Drawing of a `TessellationFrameOutput` (plan stage 1, M5): one non indexed `draw_indirect`
//! per draw bucket, the vertex shader fetches the leaf grid vertices from the storage buffers.
//! The shading is a two sided headlight lambert, there is no material or lighting integration.
//! The buckets of the leaves with `LEAF_FLAG_TRIMMED` are drawn by a separate pipeline variant
//! that runs the optional `TessellationTrimFragment` (plan-trim T2).

use parametric_tessellation::PARAM_BITS;

use super::*;
use crate::*;

/// Issue the draws of all buckets of `output`. `before` (for example the pass dispatcher that
/// declares the target formats and the depth state) and `after` (for example a clip position
/// jitter) wrap the bucket draw component of each draw. `trim` runs in the fragment stage of the
/// buckets with `LEAF_FLAG_TRIMMED`, see `TessellationTrimFragment`.
pub fn render_tessellation_output(
  ctx: &mut GPURenderPassCtx,
  output: &TessellationFrameOutput,
  uniform: &UniformBufferDataView<TessellationDrawUniform>,
  trim: Option<&dyn TessellationTrimFragment>,
  before: &[&dyn RenderComponent],
  after: &[&dyn RenderComponent],
) {
  for bucket in 0..BUCKET_COUNT as u32 {
    let draw = TessellationBucketDraw::new(output, uniform, bucket, trim);
    let mut components = before.to_vec();
    components.push(&draw);
    components.extend_from_slice(after);
    RenderSlice(&components).render(ctx, RenderMethod::TraditionalDraw(draw.draw_command()));
  }
}

/// The fragment logic of the trimming pipeline variant, it may discard the fragment or override
/// `DefaultDisplay`. Its resources (for example the trim segment pool and table) are bound in
/// `build_trim` and, in the same order, in `setup_pass`, after the resources of the bucket draw.
/// The pipeline hash includes `hash_pipeline_with_type_info`.
pub trait TessellationTrimFragment: ShaderHashProvider + ShaderPassBuilder {
  /// Runs after the shading, see `TessellationFragmentInput`.
  fn build_trim(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupBuilder,
    input: TessellationFragmentInput,
  );
}

/// The fragment inputs of a leaf (`TessLeafDraw`).
#[derive(Clone, Copy)]
pub struct TessellationFragmentInput {
  /// The perspective interpolated parameter in the patch domain `[0, 1]²`, exact at the leaf
  /// grid vertices.
  pub uv: Node<Vec2<f32>>,
  /// Indexes the tessellation item table.
  pub item: Node<u32>,
  /// Indexes `ParametricNormalizedModel::patches`.
  pub patch: Node<u32>,
}

/// How the leaves are colored, see `TessellationDrawUniform::color_mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TessellationColorMode {
  #[default]
  Gray,
  /// One palette color per k.
  ByK,
  /// A hashed color per leaf.
  ByLeaf,
  /// A hashed color per tessellation item (one (instance, patch) occurrence).
  ByPatch,
  /// The patch domain parameter as red and green.
  Parameter,
}

/// How the leaves with `LEAF_FLAG_TRIMMED` are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrimmedPatchStyle {
  /// The whole cell, the trim loops are ignored.
  Normal,
  /// The whole cell with a lighter color.
  WashedOut,
  Hidden,
  /// Trimmed by the `TessellationTrimFragment` passed to the draw (`TrimFragment`).
  #[default]
  Trimmed,
}

/// The draw settings that do not depend on the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TessellationDrawSettings {
  pub color_mode: TessellationColorMode,
  pub trimmed_style: TrimmedPatchStyle,
  /// Linear color of the gray mode.
  pub base_color: Vec3<f32>,
}

impl Default for TessellationDrawSettings {
  fn default() -> Self {
    Self {
      color_mode: Default::default(),
      trimmed_style: Default::default(),
      base_color: Vec3::splat(0.6),
    }
  }
}

#[repr(C)]
#[shader_struct(std140)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TessellationDrawUniform {
  /// Tessellation camera space (the space of `TessVertex`) to clip space.
  pub clip_from_tessellation: Mat4<f32>,
  /// Tessellation camera space to the camera space of the current view. Identity when the
  /// output was produced for the current view, otherwise the output is still drawn at the right
  /// place, only its tessellation level does not match the view.
  pub view_from_tessellation: Mat4<f32>,
  pub base_color: Vec4<f32>,
  /// `TessellationFrameOutput::bucket_capacity`.
  pub bucket_capacity: u32,
  /// `TessellationColorMode as u32`.
  pub color_mode: u32,
  /// `TrimmedPatchStyle as u32`.
  pub trimmed_style: u32,
}

impl TessellationDrawUniform {
  pub fn new(
    clip_from_tessellation: Mat4<f32>,
    view_from_tessellation: Mat4<f32>,
    settings: &TessellationDrawSettings,
    output: &TessellationFrameOutput,
  ) -> Self {
    let c = settings.base_color;
    Self {
      clip_from_tessellation,
      view_from_tessellation,
      base_color: Vec4::new(c.x, c.y, c.z, 1.),
      bucket_capacity: output.bucket_capacity,
      color_mode: settings.color_mode as u32,
      trimmed_style: settings.trimmed_style as u32,
      ..Zeroable::zeroed()
    }
  }
}

/// The colors of the `ByK` mode, indexed by `log2 k`.
pub const K_BUCKET_PALETTE: [Vec3<f32>; K_RESOLUTION_COUNT] = [
  Vec3::new(0.25, 0.45, 0.9),
  Vec3::new(0.2, 0.75, 0.7),
  Vec3::new(0.45, 0.8, 0.25),
  Vec3::new(0.95, 0.7, 0.2),
  Vec3::new(0.9, 0.3, 0.25),
];

both!(TessellationViewPosition, Vec3<f32>);
both!(TessellationViewNormal, Vec3<f32>);
both!(TessellationLeafColor, Vec3<f32>);
// the patch domain parameter of the fragment, perspective interpolated
both!(TessellationParameter, Vec2<f32>);
// the tessellation item and the patch of the fragment, flat as every integer vertex output
both!(TessellationItemIndex, u32);
both!(TessellationPatchIndex, u32);

/// The draw component of the bucket `bucket` (see `bucket_of`), the bucket is baked into the
/// pipeline. Registers `ClipPosition` in the vertex stage and `DefaultDisplay` in the fragment
/// stage, the pass dispatcher writes it to the target.
pub struct TessellationBucketDraw<'a> {
  uniform: &'a UniformBufferDataView<TessellationDrawUniform>,
  vertices: StorageBufferReadonlyDataView<[TessVertex]>,
  leaves: StorageBufferReadonlyDataView<[TessLeafDraw]>,
  bucket_leaves: StorageBufferReadonlyDataView<[u32]>,
  draw_args: GPUBufferResourceView,
  bucket: u32,
  /// Only kept for the buckets with `LEAF_FLAG_TRIMMED`.
  trim: Option<&'a dyn TessellationTrimFragment>,
}

impl<'a> TessellationBucketDraw<'a> {
  pub fn new(
    output: &TessellationFrameOutput,
    uniform: &'a UniformBufferDataView<TessellationDrawUniform>,
    bucket: u32,
    trim: Option<&'a dyn TessellationTrimFragment>,
  ) -> Self {
    assert!((bucket as usize) < BUCKET_COUNT);
    Self {
      uniform,
      vertices: output.vertices.clone().into_readonly_view(),
      leaves: output.leaves.clone().into_readonly_view(),
      bucket_leaves: output.bucket_leaves.clone().into_readonly_view(),
      draw_args: output.draw_args.gpu.clone(),
      bucket,
      trim: trim.filter(|_| bucket_is_trimmed(bucket)),
    }
  }

  pub fn draw_command(&self) -> DrawCommand {
    let size = std::mem::size_of::<DrawIndirectArgsStorage>() as u64;
    let range = GPUBufferViewRange {
      offset: self.draw_args.desc.offset + self.bucket as u64 * size,
      size: std::num::NonZeroU64::new(size),
    };
    DrawCommand::Indirect {
      indirect_buffer: self.draw_args.resource.create_view(range),
      indexed: false,
    }
  }
}

impl ShaderHashProvider for TessellationBucketDraw<'_> {
  shader_hash_type_id! {TessellationBucketDraw<'static>}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.bucket);
    hasher.hash(self.trim.is_some());
    if let Some(trim) = self.trim {
      trim.hash_pipeline_with_type_info(hasher);
    }
  }
}

impl ShaderPassBuilder for TessellationBucketDraw<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
    ctx.binding.bind(&self.vertices);
    ctx.binding.bind(&self.leaves);
    ctx.binding.bind(&self.bucket_leaves);
    if let Some(trim) = self.trim {
      trim.setup_pass(ctx);
    }
  }
}

impl GraphicsShaderProvider for TessellationBucketDraw<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    let k = bucket_k(self.bucket);
    builder.vertex(|builder, binding| {
      // two sided, the winding follows the parameter domain, not the face orientation
      builder.primitive_state().cull_mode = None;

      let uniform = binding.bind_by(self.uniform).load().expand();
      let vertices = binding.bind_by(&self.vertices);
      let leaves = binding.bind_by(&self.leaves);
      let bucket_leaves = binding.bind_by(&self.bucket_leaves);

      let instance = builder.query::<VertexInstanceIndex>();
      let vertex_index = builder.query::<VertexIndex>();
      let leaf_index = bucket_leaves
        .index(val(self.bucket) * uniform.bucket_capacity + instance)
        .load();
      let leaf = leaves.index(leaf_index).load().expand();

      // the corners (0,0) (1,0) (1,1) (0,0) (1,1) (0,1) of the cell, as bit masks over the corner
      let cell = vertex_index / val(6);
      let corner = vertex_index % val(6);
      let du = (val(0b010110_u32) >> corner) & val(1);
      let dv = (val(0b110100_u32) >> corner) & val(1);
      let i = cell % val(k) + du;
      let j = cell / val(k) + dv;
      let vertex = vertices
        .index(leaf.vertex_base + j * val(k + 1) + i)
        .load()
        .expand();

      // dyadic fixed point parameters, exact in f32 like the CPU `param_to_f64`
      let step = (val(1u32) << (val(PARAM_BITS) - leaf.level)) / val(k);
      let pu = (leaf.ix * val(k) + i) * step;
      let pv = (leaf.iy * val(k) + j) * step;
      let uv: Node<Vec2<f32>> = (param_to_f32(pu), param_to_f32(pv)).into();
      builder.set_vertex_out::<TessellationParameter>(uv);
      builder.set_vertex_out::<TessellationItemIndex>(leaf.item);
      builder.set_vertex_out::<TessellationPatchIndex>(leaf.patch);

      let trimmed = (leaf.flags & val(LEAF_FLAG_TRIMMED)).not_equals(val(0));
      let style = uniform.trimmed_style;
      let hidden = trimmed.and(style.equals(val(TrimmedPatchStyle::Hidden as u32)));
      let washed = trimmed.and(style.equals(val(TrimmedPatchStyle::WashedOut as u32)));

      // all vertices of a hidden leaf go to the same point outside the clip volume
      let clip = uniform.clip_from_tessellation * vertex.position;
      let clip = hidden.select(val(Vec4::new(0., 0., -2., 1.)), clip);
      builder.register::<ClipPosition>(clip);

      let position = (uniform.view_from_tessellation * vertex.position).xyz();
      let normal = (uniform.view_from_tessellation * vertex.normal).xyz();
      builder.set_vertex_out::<TessellationViewPosition>(position);
      builder.set_vertex_out::<TessellationViewNormal>(normal);

      let mode = uniform.color_mode;
      let k_color = val(K_BUCKET_PALETTE[k.trailing_zeros() as usize]);
      let color = mode
        .equals(val(TessellationColorMode::ByK as u32))
        .select(k_color, uniform.base_color.xyz());
      let color = mode
        .equals(val(TessellationColorMode::ByLeaf as u32))
        .select(hash_color(leaf_index), color);
      let color = mode
        .equals(val(TessellationColorMode::ByPatch as u32))
        .select(hash_color(leaf.item), color);
      let parameter_color: Node<Vec3<f32>> = (uv, val(0.)).into();
      let color = mode
        .equals(val(TessellationColorMode::Parameter as u32))
        .select(parameter_color, color);
      let color = washed.select(val(0.6).mix(color, val(Vec3::one())), color);
      builder.set_vertex_out::<TessellationLeafColor>(color);
    });

    builder.fragment(|builder, binding| {
      let position = builder.query::<TessellationViewPosition>();
      let normal = builder.query::<TessellationViewNormal>();
      let color = builder.query::<TessellationLeafColor>();

      // the headlight shines along the view ray, abs makes it two sided
      let to_eye = -position.normalize();
      let normal_length = normal.length();
      let normal = normal_length
        .greater_than(0.)
        .select(normal / normal_length, to_eye);
      let lambert = normal.dot(to_eye).abs();
      let shade = val(0.15) + val(0.85) * lambert;
      let display: Node<Vec4<f32>> = (color * shade, val(1.)).into();
      builder.register::<DefaultDisplay>(display);

      if let Some(trim) = self.trim {
        let input = TessellationFragmentInput {
          uv: builder.query::<TessellationParameter>(),
          item: builder.query::<TessellationItemIndex>(),
          patch: builder.query::<TessellationPatchIndex>(),
        };
        trim.build_trim(builder, binding, input);
      }
    });
  }
}

/// A bright pseudo random color of `id`.
fn hash_color(id: Node<u32>) -> Node<Vec3<f32>> {
  let h = id * val(0x9E37_79B9_u32);
  let h = h ^ (h >> val(16));
  let h = h * val(0x85EB_CA6B_u32);
  let h = h ^ (h >> val(13));
  let rgb: Node<Vec3<u32>> = (
    h & val(255),
    (h >> val(8)) & val(255),
    (h >> val(16)) & val(255),
  )
    .into();
  rgb.into_f32() * val(0.7 / 255.) + val(Vec3::splat(0.3))
}
