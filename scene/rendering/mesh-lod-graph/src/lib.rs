use database::EntityHandle;
use rendiation_device_parallel_compute::DeviceParallelComputeCtx;
use rendiation_scene_core::*;
use rendiation_scene_rendering_gpu_base::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod lod;
use lod::*;
mod meta;
use meta::*;
mod draw;
use draw::*;
mod expand;
use expand::*;

pub struct MeshLODGraphRenderer {
  pub mesh_src_data: StorageBufferReadonlyDataView<[MeshletMeshMetaData]>,
  pub position_buffer: StorageBufferReadonlyDataView<[u32]>,
  pub meshlet_buffer: StorageBufferReadonlyDataView<[u32]>,
  pub index_buffer: StorageBufferReadonlyDataView<[u32]>,
}

impl MeshLODGraphRenderer {
  pub fn prepare_draw(
    &self,
    batch: &SceneModelRenderBatch,
    cx: &mut DeviceParallelComputeCtx,
  ) -> Vec<Box<dyn IndirectDrawProvider>> {
    let device_batch = batch.get_device_batch(None).unwrap();

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

        batch.create_indirect_draw_provider(builder, cx)
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
