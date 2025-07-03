use crate::*;

pub type WideLineUniforms =
  UniformUpdateContainer<EntityHandle<WideLineModelEntity>, WideLineUniform>;

pub fn wide_line_instance_buffers(
  cx: &GPU,
) -> impl ReactiveValueRefQuery<Key = EntityHandle<WideLineModelEntity>, Value = GPUBufferResourceView>
{
  let cx = cx.clone();
  global_watch()
    .watch::<WideLineMeshBuffer>()
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      move |_, buffer| {
        create_gpu_buffer(buffer.as_slice(), BufferUsages::VERTEX, &cx.device) //
          .create_default_view()
      }
    })
    .materialize_unordered()
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct WideLineUniform {
  pub width: f32,
}

pub struct WideLineGPU<'a> {
  pub uniform: &'a UniformBufferDataView<WideLineUniform>,
  pub index_buffer: &'a GPUBufferResourceView,
  pub vertex_buffer: &'a GPUBufferResourceView,
  pub instance_buffer: &'a GPUBufferResourceView,
}

impl ShaderHashProvider for WideLineGPU<'_> {
  shader_hash_type_id! {WideLineGPU<'static>}
}

impl ShaderPassBuilder for WideLineGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx
      .pass
      .set_index_buffer_by_buffer_resource_view(self.index_buffer, IndexFormat::Uint16);
    ctx.set_vertex_buffer_by_buffer_resource_view_next(self.vertex_buffer);
    ctx.set_vertex_buffer_by_buffer_resource_view_next(self.instance_buffer);

    ctx.binding.bind(self.uniform);
  }
}

impl GraphicsShaderProvider for WideLineGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
      builder.register_vertex::<CommonVertex>(VertexStepMode::Vertex);
      builder.register_vertex::<WideLineVertex>(VertexStepMode::Instance);
      builder.primitive_state.topology = rendiation_webgpu::PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;

      let uv = builder.query::<GeometryUV>();
      let color_with_alpha = builder.query::<GeometryColorWithAlpha>();
      let material = binding.bind_by(&self.uniform).load().expand();

      let vertex_position = wide_line_vertex(
        builder.query::<WideLineStart>(),
        builder.query::<WideLineEnd>(),
        builder.query::<GeometryPosition>(),
        builder.query::<RenderBufferSize>(),
        material.width,
        builder,
      );

      builder.register::<ClipPosition>(vertex_position);
      builder.set_vertex_out::<FragmentUv>(uv);
      builder.set_vertex_out::<DefaultDisplay>(color_with_alpha);
    });

    builder.fragment(|builder, _| {
      let uv = builder.query::<FragmentUv>();
      if_by(discard_round_corner(uv), || {
        builder.discard();
      });
    })
  }
}

fn wide_line_vertex(
  wide_line_start: Node<Vec3<f32>>,
  wide_line_end: Node<Vec3<f32>>,
  position: Node<Vec3<f32>>,
  view_size: Node<Vec2<f32>>,
  width: Node<f32>,
  builder: &mut ShaderVertexBuilder,
) -> Node<Vec4<f32>> {
  let object_world_position = builder.query::<WorldPositionHP>();
  let (clip_start, _) = camera_transform_impl(builder, wide_line_start, object_world_position);
  let (clip_end, _) = camera_transform_impl(builder, wide_line_end, object_world_position);

  let aspect = view_size.x() / view_size.y();

  // ndc space
  let ndc_start = clip_start.xy() / clip_start.w().splat();
  let ndc_end = clip_end.xy() / clip_end.w().splat();

  // direction
  let dir = ndc_end - ndc_start;

  // account for clip-space aspect ratio
  let dir = vec2_node((dir.x() * aspect, dir.y()));
  let dir = dir.normalize();

  // perpendicular to dir
  let offset = vec2_node((dir.y(), -dir.x()));

  // undo aspect ratio adjustment
  let dir = vec2_node((dir.x() / aspect, dir.y()));
  let offset = vec2_node((offset.x() / aspect, offset.y()));
  let offset = offset.make_local_var();

  // sign flip
  if_by(position.x().less_than(0.), || {
    offset.store(-offset.load());
  });

  // end caps
  if_by(position.y().less_than(0.), || {
    offset.store(offset.load() - dir);
  });

  if_by(position.y().greater_than(1.), || {
    offset.store(offset.load() + dir);
  });

  let mut offset = offset.load();

  // adjust for width
  offset *= width.splat();
  // adjust for clip-space to screen-space conversion // maybe resolution should be based on
  // viewport ...
  offset /= view_size.y().splat();

  // select end
  let clip = position.y().less_than(0.5).select(clip_start, clip_end);

  // back to clip space
  offset = offset * clip.w();
  (clip.xy() + offset, clip.zw()).into()
}

fn discard_round_corner(uv: Node<Vec2<f32>>) -> Node<bool> {
  let a = uv.x();
  let b = uv.y() + uv.y().greater_than(0.).select(-1., 1.);
  let len2 = a * a + b * b;

  uv.y().abs().greater_than(1.).and(len2.greater_than(1.))
}

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod, ShaderVertex)]
pub struct WideLineVertex {
  #[semantic(WideLineStart)]
  pub start: Vec3<f32>,
  #[semantic(WideLineEnd)]
  pub end: Vec3<f32>,
  #[semantic(GeometryColorWithAlpha)]
  pub color: Vec4<f32>,
}

only_vertex!(WideLineStart, Vec3<f32>);
only_vertex!(WideLineEnd, Vec3<f32>);

fn create_wide_line_quad() -> IndexedMesh<TriangleList, Vec<CommonVertex>, Vec<u16>> {
  #[rustfmt::skip]
  let positions: Vec<isize> = vec![- 1, 2, 0, 1, 2, 0, - 1, 1, 0, 1, 1, 0, - 1, 0, 0, 1, 0, 0, - 1, - 1, 0, 1, - 1, 0];
  let positions: &[Vec3<isize>] = bytemuck::cast_slice(positions.as_slice());
  let uvs: Vec<isize> = vec![-1, 2, 1, 2, -1, 1, 1, 1, -1, -1, 1, -1, -1, -2, 1, -2];
  let uvs: &[Vec2<isize>] = bytemuck::cast_slice(uvs.as_slice());

  let data: Vec<_> = positions
    .iter()
    .zip(uvs)
    .map(|(position, uv)| CommonVertex {
      position: position.map(|v| v as f32),
      normal: Vec3::new(0., 0., 1.),
      uv: uv.map(|v| v as f32),
    })
    .collect();

  let index = vec![0, 2, 1, 2, 3, 1, 2, 4, 3, 4, 5, 3, 4, 6, 5, 6, 7, 5];
  IndexedMesh::new(data, index)
}

pub fn create_wide_line_quad_gpu(gpu: &GPU) -> (GPUBufferResourceView, GPUBufferResourceView) {
  let line = create_wide_line_quad();

  let index = create_gpu_buffer(
    bytemuck::cast_slice(&line.index),
    BufferUsages::INDEX,
    &gpu.device,
  )
  .create_default_view();
  let vertex = create_gpu_buffer(
    bytemuck::cast_slice(&line.vertex),
    BufferUsages::VERTEX,
    &gpu.device,
  )
  .create_default_view();
  (index, vertex)
}
