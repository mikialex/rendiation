use database::*;
use reactive::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_reactive_utils::*;

mod shape;
pub use shape::*;

pub fn register_wide_line_data_model() {
  global_entity_of::<SceneModelEntity>() //
    .declare_foreign_key::<SceneModelWideLineRenderPayload>();

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
  ExternalRefPtr<Vec<u8>>
);

pub struct WideLineMeshDataView {
  pub width: f32,
  pub buffer: WideLineMeshInternal,
}

pub type WideLineMeshInternal = NoneIndexedMesh<LineList, Vec<WideLineVertex>>;

pub fn use_widen_line(qcx: &mut impl QueryGPUHookCx) -> Option<WideLineModelRenderer> {
  let (qcx, quad) = qcx.use_gpu_init(create_wide_line_quad_gpu);

  let uniform =
    qcx.use_uniform_buffers::<EntityHandle<WideLineModelEntity>, WideLineUniform>(|source, cx| {
      let width = global_watch()
        .watch::<WideLineWidth>()
        .into_query_update_uniform(offset_of!(WideLineUniform, width), cx);

      source.with_source(width)
    });

  let mesh = qcx.use_val_refed_reactive_query(wide_line_instance_buffers);

  qcx.when_render(|| WideLineModelRenderer {
    model_access: global_database().read_foreign_key::<SceneModelWideLineRenderPayload>(),
    uniforms: uniform.unwrap(),
    instance_buffers: mesh.unwrap(),
    index_buffer: quad.0.clone(),
    vertex_buffer: quad.1.clone(),
  })
}

pub struct WideLineModelRenderer {
  model_access: ForeignKeyReadView<SceneModelWideLineRenderPayload>,
  uniforms: LockReadGuardHolder<WideLineUniforms>,
  instance_buffers: BoxedDynValueRefQuery<EntityHandle<WideLineModelEntity>, GPUBufferResourceView>,
  index_buffer: GPUBufferResourceView,
  vertex_buffer: GPUBufferResourceView,
}

impl GLESModelRenderImpl for WideLineModelRenderer {
  fn shape_renderable(
    &self,
    idx: EntityHandle<SceneModelEntity>,
  ) -> Option<(Box<dyn RenderComponent + '_>, DrawCommand)> {
    let mesh_idx = self.model_access.get(idx)?;
    let uniform = self.uniforms.get(&mesh_idx).unwrap();
    let instance_buffer = self.instance_buffers.access_ref(&mesh_idx).unwrap();

    let instance_count = u64::from(instance_buffer.view_byte_size()) as usize
      / std::mem::size_of::<WideLineVertex>()
      / 2;
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
