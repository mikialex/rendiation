use crate::*;

pub fn use_widen_line_gles_renderer(cx: &mut QueryGPUHookCx) -> Option<WideLineModelGLESRenderer> {
  let (cx, quad) = cx.use_gpu_init(|g, _| create_wide_line_quad_gpu(g));

  let uniform = cx.use_uniform_buffers();

  let offset = offset_of!(WideLineUniform, width);
  cx.use_changes::<WideLineWidth>()
    .update_uniforms(&uniform, offset, cx.gpu);

  let offset = offset_of!(WideLineUniform, style_factor);
  cx.use_changes::<WideLineStyleFactor>()
    .update_uniforms(&uniform, offset, cx.gpu);

  let offset = offset_of!(WideLineUniform, style_pattern);
  cx.use_changes::<WideLineStylePattern>()
    .update_uniforms(&uniform, offset, cx.gpu);

  let offset = offset_of!(WideLineUniform, enable_round_joint);
  cx.use_changes::<WideLineEnableRoundJoint>()
    .map_changes(Bool::from)
    .update_uniforms(&uniform, offset, cx.gpu);

  let offset = offset_of!(WideLineUniform, color);
  cx.use_changes::<WideLineColor>()
    .update_uniforms(&uniform, offset, cx.gpu);

  let mesh = cx.use_shared_hash_map("wide line mesh gles");

  maintain_shared_map(&mesh, cx.use_changes::<WideLineMeshBuffer>(), |buffer| {
    let buffer = create_gpu_buffer(&buffer, BufferUsages::VERTEX, &cx.gpu.device);
    buffer.create_default_view()
  });

  cx.when_render(|| WideLineModelGLESRenderer {
    model_access: global_database().read_foreign_key(),
    uniforms: uniform.make_read_holder(),
    instance_buffers: mesh.make_read_holder(),
    index_buffer: quad.0.clone(),
    vertex_buffer: quad.1.clone(),
  })
}

pub struct WideLineModelGLESRenderer {
  model_access: ForeignKeyReadView<SceneModelWideLineRenderPayload>,
  uniforms: LockReadGuardHolder<WideLineUniforms>,
  instance_buffers: SharedHashMapRead<u32, GPUBufferResourceView>,
  index_buffer: GPUBufferResourceView,
  vertex_buffer: GPUBufferResourceView,
}

impl GLESModelRenderImpl for WideLineModelGLESRenderer {
  fn shape_renderable<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    _cx: &'a GPUTextureBindingSystem,
  ) -> Option<(Box<dyn RenderComponent + 'a>, DrawCommand)> {
    let model_idx = self.model_access.get(idx)?;
    let uniform = self.uniforms.get(&model_idx.alloc_index()).unwrap();
    let instance_buffer = self
      .instance_buffers
      .access_ref(&model_idx.alloc_index())
      .unwrap();

    let instance_count =
      u64::from(instance_buffer.view_byte_size()) as usize / std::mem::size_of::<WideLineVertex>();

    let draw_command = DrawCommand::Indexed {
      instances: 0..instance_count as u32,
      base_vertex: 0,
      indices: 0..18,
    };

    let com = Box::new(WideLineGPU {
      uniform,
      vertex_buffer: &self.vertex_buffer,
      index_buffer: &self.index_buffer,
      instance_buffer,
    });
    Some((com, draw_command))
  }
  fn material_renderable<'a>(
    &'a self,
    _idx: EntityHandle<SceneModelEntity>,
    _cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    Some(Box::new(())) // no material
  }
}

type WideLineUniforms = UniformBufferCollectionRaw<u32, WideLineUniform>;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct WideLineUniform {
  pub width: f32,
  pub style_factor: f32,
  pub style_pattern: u32,
  pub enable_round_joint: Bool,
  pub color: Vec4<f32>,
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
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
  }
}

impl GraphicsShaderProvider for WideLineGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, _| {
      builder.register_vertex::<CommonVertex>(VertexStepMode::Vertex);
      builder.register_vertex::<WideLineVertex>(VertexStepMode::Instance);
      builder.primitive_state.topology = rendiation_webgpu::PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;
    });
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    let mut uniform = BindingPreparer::new(&self.uniform);

    builder.vertex(|builder, binding| {
      let uv = builder.query::<GeometryUV>();
      let color_with_alpha = builder.query::<GeometryColorWithAlpha>();
      let uniform = uniform.using(binding).load().expand();

      wide_line_vertex(
        builder.query::<WideLineStart>(),
        builder.query::<WideLineEnd>(),
        builder.query::<GeometryPosition>(),
        builder.query::<ViewportRenderBufferSize>(),
        uniform.width,
        builder,
      );

      builder.set_vertex_out::<FragmentUv>(uv);
      builder.set_vertex_out::<DefaultDisplay>(color_with_alpha * uniform.color);
    });

    builder.fragment(|builder, binding| {
      let uv = builder.query::<FragmentUv>();
      builder.insert_type_tag::<UnlitMaterialTag>();
      let line_param = uniform.using(binding).load().expand();

      let enable_line_pattern = line_param.style_pattern.not_equals(val(0));
      let coord = builder.query::<FragmentPosition>().xy();
      let sc_coord = builder.query::<WideLineScreenCoord>();
      let should_discard_by_pattern = enable_line_pattern.and(discard_by_line_pattern_fn(
        line_param.style_factor,
        line_param.style_pattern,
        sc_coord,
        coord,
      ));
      if_by(should_discard_by_pattern, || {
        builder.discard();
      });

      let enable_round_joint = line_param.enable_round_joint.into_bool();
      let should_discard_by_joint_style = enable_round_joint.and(discard_by_round_corner_fn(uv));
      if_by(should_discard_by_joint_style, || {
        builder.discard();
      });
    })
  }
}

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
