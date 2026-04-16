use rendiation_scene_rendering_gpu_gles::*;

use crate::*;

pub fn use_widen_points_gles_renderer(
  cx: &mut QueryGPUHookCx,
) -> Option<WidePointModelGLESRenderer> {
  let (cx, quad) = cx.use_gpu_init(|g, _| create_wide_point_quad_gpu(g));

  let uniform = cx.use_uniform_buffers();

  cx.use_changes::<WideStyledPointsColor>().update_uniforms(
    &uniform,
    offset_of!(WidePointUniform, color),
    cx.gpu,
  );

  let color_alpha_texture = offset_of!(WidePointUniform, color_alpha_texture);
  use_tex_watcher::<WideLineColorAlphaTex, _>(cx, color_alpha_texture, &uniform);

  let mesh = cx.use_shared_hash_map("wide point mesh gles");

  maintain_shared_map(
    &mesh,
    cx.use_changes::<WideStyledPointsMeshBuffer>(),
    |buffer| {
      let buffer = create_gpu_buffer(&buffer, BufferUsages::VERTEX, &cx.gpu.device);
      buffer.create_default_view()
    },
  );

  cx.when_render(|| WidePointModelGLESRenderer {
    model_access: global_database().read_foreign_key(),
    uniforms: uniform.make_read_holder(),
    instance_buffers: mesh.make_read_holder(),
    index_buffer: quad.0.clone(),
    vertex_buffer: quad.1.clone(),
    tex: TextureSamplerIdView::read_from_global(),
  })
}

pub struct WidePointModelGLESRenderer {
  model_access: ForeignKeyReadView<SceneModelWideStyledPointsRenderPayload>,
  tex: TextureSamplerIdView<WideLineColorAlphaTex>,
  uniforms: LockReadGuardHolder<WidePointUniforms>,
  instance_buffers: SharedHashMapRead<u32, GPUBufferResourceView>,
  index_buffer: GPUBufferResourceView,
  vertex_buffer: GPUBufferResourceView,
}

impl GLESModelRenderImpl for WidePointModelGLESRenderer {
  fn shape_renderable<'a>(
    &'a self,
    idx: EntityHandle<SceneModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<(Box<dyn RenderComponent + 'a>, DrawCommand)> {
    let model_idx = self.model_access.get(idx)?;
    let uniform = self.uniforms.get(&model_idx.alloc_index()).unwrap();
    let instance_buffer = self
      .instance_buffers
      .access_ref(&model_idx.alloc_index())
      .unwrap();

    let instance_count = u64::from(instance_buffer.view_byte_size()) as usize
      / std::mem::size_of::<WideStyledPointVertex>();

    let draw_command = DrawCommand::Indexed {
      instances: 0..instance_count as u32,
      base_vertex: 0,
      indices: 0..6,
    };

    let com = Box::new(WidePointGPU {
      uniform,
      vertex_buffer: &self.vertex_buffer,
      index_buffer: &self.index_buffer,
      instance_buffer,
      color_alpha_tex_sampler: self.tex.get_pair(model_idx).unwrap_or(EMPTY_H),
      binding_sys: cx,
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

type WidePointUniforms = UniformBufferCollectionRaw<u32, WidePointUniform>;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct WidePointUniform {
  pub color: Vec3<f32>,
  pub color_alpha_texture: TextureSamplerHandlePair,
}

pub struct WidePointGPU<'a> {
  uniform: &'a UniformBufferDataView<WidePointUniform>,
  index_buffer: &'a GPUBufferResourceView,
  vertex_buffer: &'a GPUBufferResourceView,
  instance_buffer: &'a GPUBufferResourceView,
  color_alpha_tex_sampler: (u32, u32),
  binding_sys: &'a GPUTextureBindingSystem,
}

impl ShaderHashProvider for WidePointGPU<'_> {
  shader_hash_type_id! {WidePointGPU<'static>}
}

impl ShaderPassBuilder for WidePointGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx
      .pass
      .set_index_buffer_by_buffer_resource_view(self.index_buffer, IndexFormat::Uint16);
    ctx.set_vertex_buffer_by_buffer_resource_view_next(self.vertex_buffer);
    ctx.set_vertex_buffer_by_buffer_resource_view_next(self.instance_buffer);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.uniform);
    setup_tex(ctx, self.binding_sys, self.color_alpha_tex_sampler);
  }
}

impl GraphicsShaderProvider for WidePointGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, _| {
      builder.register_vertex::<CommonVertex>(VertexStepMode::Vertex);
      builder.register_vertex::<WideStyledPointVertex>(VertexStepMode::Instance);
      builder.primitive_state.topology = rendiation_webgpu::PrimitiveTopology::TriangleList;
      builder.primitive_state.cull_mode = None;

      let style_id = builder.query::<WidePointStyleId>();
      builder.set_vertex_out::<WidePointStyleId>(style_id);
    });
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, _| {
      let uv = builder.query::<GeometryUV>();
      let width = builder.query::<WidePointSize>();

      wide_line_vertex(
        builder.query::<WidePointPosition>(),
        builder.query::<GeometryPosition>(),
        builder.query::<ViewportRenderBufferSize>(),
        width,
        builder,
      );

      builder.set_vertex_out::<FragmentUv>(uv);
    });

    builder.fragment(|builder, binding| {
      let uv = builder.query::<FragmentUv>();
      // reject lighting
      builder.insert_type_tag::<UnlitMaterialTag>();

      let coord = uv * val(Vec2::new(2., 2.)) - val(Vec2::new(1., 1.));
      let style_id = builder.query::<WidePointStyleId>();

      let uniform = binding.bind_by(&self.uniform).load().expand();
      let color = uniform.color;

      let (alpha, color_multiplier) = point_style_entry(coord, style_id);

      let color_alpha_tex = bind_and_sample(
        self.binding_sys,
        binding,
        builder.registry(),
        self.color_alpha_tex_sampler,
        uniform.color_alpha_texture,
        uv,
        val(Vec4::one()),
      );

      let alpha = alpha * color_alpha_tex.w();
      let color_multiplier = color_multiplier * color_alpha_tex.xyz();

      let final_color: Node<Vec4<f32>> = (color * color_multiplier, alpha).into();

      builder.register::<DefaultDisplay>(final_color);

      builder.frag_output.iter_mut().for_each(|p| {
        if p.is_blendable() {
          p.states.blend = BlendState::ALPHA_BLENDING.into();
        }
      });
      if let Some(depth) = &mut builder.depth_stencil {
        depth.depth_write_enabled = false;
      }
    })
  }
}

fn create_wide_point_quad() -> IndexedMesh<TriangleList, Vec<CommonVertex>, Vec<u16>> {
  #[rustfmt::skip]
  let positions: Vec<isize> = vec![0, 0,  1, 1,  1, 0,   0, 0,  0,1, 1, 1];
  let positions: &[Vec2<isize>] = bytemuck::cast_slice(positions.as_slice());

  let data: Vec<_> = positions
    .iter()
    .map(|position| CommonVertex {
      position: Vec3::new(position.x as f32, position.y as f32, 0.),
      normal: Vec3::new(0., 0., 1.),
      uv: position.map(|v| v as f32),
    })
    .collect();

  let index = vec![0, 1, 2, 3, 4, 5];
  IndexedMesh::new(data, index)
}

pub fn create_wide_point_quad_gpu(gpu: &GPU) -> (GPUBufferResourceView, GPUBufferResourceView) {
  let point = create_wide_point_quad();

  let index = create_gpu_buffer(
    bytemuck::cast_slice(&point.index),
    BufferUsages::INDEX,
    &gpu.device,
  )
  .create_default_view();
  let vertex = create_gpu_buffer(
    bytemuck::cast_slice(&point.vertex),
    BufferUsages::VERTEX,
    &gpu.device,
  )
  .create_default_view();
  (index, vertex)
}
