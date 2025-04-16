use database::EntityHandle;
use rendiation_device_parallel_compute::DeviceParallelComputeCtx;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod lod;
use lod::*;
mod draw_access;
use draw_access::*;
mod draw_prepare;
use draw_prepare::*;

#[repr(C)]
#[std430_layout]
#[derive(Debug, Clone, Copy, ShaderStruct, PartialEq)]
pub struct MeshletMetaData {
  pub index_offset: u32,
  pub index_count: u32,
  pub position_offset: u32,
  pub bounds: LODBoundPair,
}

impl Default for MeshletMetaData {
  fn default() -> Self {
    Self {
      index_offset: u32::MAX,
      position_offset: u32::MAX,
      ..Zeroable::zeroed()
    }
  }
}

pub struct MeshLODGraphRenderer {
  pub mesh_src_data: StorageBufferReadonlyDataView<[MeshletMetaData]>,
  pub scene_model_meshlet_range: StorageBufferDataView<[Vec2<u32>]>,
  pub position_buffer: StorageBufferReadonlyDataView<[u32]>,
  pub meshlet_buffer: StorageBufferReadonlyDataView<[u32]>,
  pub index_buffer: StorageBufferReadonlyDataView<[u32]>,
}

impl MeshLODGraphRenderer {
  pub fn prepare_draw(
    &self,
    batch: &SceneModelRenderBatch,
    cx: &mut DeviceParallelComputeCtx,
    lod_decider: LODDecider,
  ) -> Vec<Box<dyn IndirectDrawProvider>> {
    let device_batch = batch.get_device_batch(None).unwrap();

    let lod_decider = create_uniform(lod_decider, &cx.gpu.device);

    device_batch
      .sub_batches
      .iter()
      .map(|batch| {
        let builder = DrawCommandBuilder::Indexed(Box::new(MeshletDrawCommandBuilder {}));

        // let drawer = Box::new(MeshletGPUDraw {
        //   position_buffer: self.position_buffer.clone(),
        //   mesh_src_data: self.mesh_src_data.clone(),
        //   index_buffer: self.index_buffer.clone(),
        // });

        batch.create_default_indirect_draw_provider(builder, cx)
      })
      .collect()
  }
}

#[derive(Clone)]
struct MeshletDrawCommandBuilder {}

impl ShaderHashProvider for MeshletDrawCommandBuilder {
  shader_hash_type_id! {}
}
impl ShaderPassBuilder for MeshletDrawCommandBuilder {}

impl IndexedDrawCommandBuilder for MeshletDrawCommandBuilder {
  fn draw_command_host_access(&self, _id: EntityHandle<SceneModelEntity>) -> DrawCommand {
    unimplemented!("host access is not supported")
  }

  fn build_invocation(
    &self,
    cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn IndexedDrawCommandBuilderInvocation> {
    Box::new(MeshletDrawCommandInvocation {})
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    todo!()
  }
}

struct MeshletDrawCommandInvocation {}

impl IndexedDrawCommandBuilderInvocation for MeshletDrawCommandInvocation {
  fn generate_draw_command(&self, draw_id: Node<u32>) -> Node<DrawIndexedIndirect> {
    let sm_id: Node<u32> = todo!(); // extract from packed draw_id;
    let meshlet_id: Node<u32> = todo!();
    todo!()
  }
}
