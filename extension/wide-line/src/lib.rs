use database::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod shape;
pub use shape::*;

pub fn register_wide_line_data_model() {
  global_entity_of::<SceneModelEntity>() //
    .declare_sparse_foreign_key::<SceneModelWideLineRenderPayload>();

  global_database()
    .declare_entity::<WideLineModelEntity>()
    .declare_component::<WideLineWidth>()
    .declare_component::<WideLineMeshBuffer>();
}

declare_foreign_key!(
  SceneModelWideLineRenderPayload,
  SceneModelEntity,
  WideLineModelEntity
);

declare_entity!(WideLineModelEntity);
declare_component!(WideLineWidth, WideLineModelEntity, f32, 1.0);
declare_component!(
  WideLineMeshBuffer,
  WideLineModelEntity,
  ExternalRefPtr<Vec<u8>> // Vec<WideLineVertex>
);

pub struct WideLineMeshDataView {
  pub width: f32,
  pub buffer: WideLineMeshInternal,
}

pub type WideLineMeshInternal = NoneIndexedMesh<LineList, Vec<WideLineVertex>>;

pub fn use_widen_line_gles_renderer(cx: &mut QueryGPUHookCx) -> Option<WideLineModelRenderer> {
  let (cx, quad) = cx.use_gpu_init(|g, _| create_wide_line_quad_gpu(g));

  let uniform = cx.use_uniform_buffers();

  cx.use_changes::<WideLineWidth>().update_uniforms(
    &uniform,
    offset_of!(WideLineUniform, width),
    cx.gpu,
  );

  let mesh = cx.use_shared_hash_map();

  maintain_shared_map(&mesh, cx.use_changes::<WideLineMeshBuffer>(), |buffer| {
    let buffer = create_gpu_buffer(&buffer, BufferUsages::VERTEX, &cx.gpu.device);
    buffer.create_default_view()
  });

  cx.when_render(|| WideLineModelRenderer {
    model_access: global_database().read_foreign_key::<SceneModelWideLineRenderPayload>(),
    uniforms: uniform.make_read_holder(),
    instance_buffers: mesh.make_read_holder(),
    index_buffer: quad.0.clone(),
    vertex_buffer: quad.1.clone(),
  })
}

pub struct WideLineModelRenderer {
  model_access: ForeignKeyReadView<SceneModelWideLineRenderPayload>,
  uniforms: LockReadGuardHolder<WideLineUniforms>,
  instance_buffers: SharedHashMapRead<u32, GPUBufferResourceView>,
  index_buffer: GPUBufferResourceView,
  vertex_buffer: GPUBufferResourceView,
}

impl GLESModelRenderImpl for WideLineModelRenderer {
  fn shape_renderable(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
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
