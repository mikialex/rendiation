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

only_vertex!(ShaderMat4VertexColumOne, Vec4<f32>);
only_vertex!(ShaderMat4VertexColumTwo, Vec4<f32>);
only_vertex!(ShaderMat4VertexColumThree, Vec4<f32>);
only_vertex!(ShaderMat4VertexColumFour, Vec4<f32>);
#[repr(C)]
#[derive(Clone, Copy, shadergraph::ShaderVertex)]
pub struct ShaderMat4VertexInput {
  #[semantic(ShaderMat4VertexColumOne)]
  colum1: Vec4<f32>,
  #[semantic(ShaderMat4VertexColumTwo)]
  colum2: Vec4<f32>,
  #[semantic(ShaderMat4VertexColumThree)]
  colum3: Vec4<f32>,
  #[semantic(ShaderMat4VertexColumFour)]
  colum4: Vec4<f32>,
}

impl<M: WebGPUMesh> ShaderGraphProvider for TransformInstanceGPU<M> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.mesh_gpu.build(builder)?;
    builder.vertex(|builder, _| {
      builder.register_vertex::<ShaderMat4VertexInput>(VertexStepMode::Instance);

      let c1 = builder.query::<ShaderMat4VertexColumOne>()?.get();
      let c2 = builder.query::<ShaderMat4VertexColumTwo>()?.get();
      let c3 = builder.query::<ShaderMat4VertexColumThree>()?.get();
      let c4 = builder.query::<ShaderMat4VertexColumFour>()?.get();
      let world_mat: Node<Mat4<f32>> = (c1, c2, c3, c4).into();
      let world_normal_mat: Node<Mat3<f32>> = (c1.xyz(), c2.xyz(), c3.xyz()).into();
      let world_normal_mat = world_normal_mat.inverse().transpose();

      if let Ok(position) = builder.query::<GeometryPosition>() {
        position.set(world_mat * position.get())
      }

      if let Ok(normal) = builder.query::<GeometryNormal>() {
        normal.set(world_normal_mat * normal.get())
      }

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
    *gpu_mesh = self.create(gpu, storage)
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
