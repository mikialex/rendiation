use std::sync::Arc;

use database::*;
use fast_hash_collection::*;
use parking_lot::RwLock;
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
  ExternalRefPtr<Vec<u8>>
);

pub struct WideLineMeshDataView {
  pub width: f32,
  pub buffer: WideLineMeshInternal,
}

pub type WideLineMeshInternal = NoneIndexedMesh<LineList, Vec<WideLineVertex>>;

type BufferCollection = Arc<RwLock<FastHashMap<u32, GPUBufferResourceView>>>;
type BufferCollectionRead = LockReadGuardHolder<FastHashMap<u32, GPUBufferResourceView>>;

pub fn use_widen_line(qcx: &mut QueryGPUHookCx) -> Option<WideLineModelRenderer> {
  let (qcx, quad) = qcx.use_gpu_init(create_wide_line_quad_gpu);

  let uniform = qcx.use_uniform_buffers();

  qcx.use_changes::<WideLineWidth>().update_uniforms(
    &uniform,
    offset_of!(WideLineUniform, width),
    qcx.gpu,
  );

  let (qcx, mesh) = qcx.use_plain_state_default_cloned::<BufferCollection>();

  if let Some(changes) = qcx.use_changes::<WideLineMeshBuffer>().if_ready() {
    if changes.has_change() {
      let mut map = mesh.write();
      for k in changes.iter_removed() {
        map.remove(&k);
      }
      for (k, buffer) in changes.iter_update_or_insert() {
        let buffer = create_gpu_buffer(buffer.as_slice(), BufferUsages::VERTEX, &qcx.gpu.device);
        let buffer = buffer.create_default_view();
        map.insert(k, buffer);
      }
    }
  }

  qcx.when_render(|| WideLineModelRenderer {
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
  instance_buffers: BufferCollectionRead,
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
