use rendiation_algebra::*;
use std::rc::Rc;

use crate::*;

pub struct TransformInstance<M> {
  mesh: M,
  transforms: Vec<Mat4<f32>>,
}

pub struct TransformInstanceGPU<M: WebGPUMesh> {
  mesh_gpu: M::GPU,
  instance_gpu: Rc<webgpu::Buffer>,
}

impl<M: WebGPUMesh> ShaderGraphProvider for TransformInstanceGPU<M> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.mesh_gpu.build(builder);
    builder.vertex(|builder, _| {
      // todo, override the world_position staff
      Ok(())
    })
  }
}

impl<M: WebGPUMesh> ShaderHashProvider for TransformInstanceGPU<M> {}

impl<M: WebGPUMesh> ShaderPassBuilder for TransformInstanceGPU<M> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.mesh_gpu.setup_pass(ctx)
    // todo set_vertex_buffer_owned for self, the index should be considered
  }
}

impl<M: WebGPUMesh> WebGPUMesh for TransformInstance<M> {
  type GPU = TransformInstanceGPU<M>;

  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &webgpu::GPU, storage: &mut anymap::AnyMap) {
    todo!()
  }

  fn create(&self, gpu: &webgpu::GPU, storage: &mut anymap::AnyMap) -> Self::GPU {
    let mesh_gpu = self.mesh.create(gpu, storage);
    TransformInstanceGPU {
      mesh_gpu,
      instance_gpu: todo!(),
    }
  }

  // we should constrain this call
  fn draw_impl<'a>(
    &self,
    pass: &mut webgpu::GPURenderPass<'a>,
    group: rendiation_renderable_mesh::group::MeshDrawGroup,
  ) {
    todo!()
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.mesh.topology()
  }

  fn try_pick(
    &self,
    f: &mut dyn FnMut(&dyn rendiation_renderable_mesh::mesh::IntersectAbleGroupedMesh),
  ) {
  }
}
