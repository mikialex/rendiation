use rendiation_algebra::*;
use rendiation_renderable_mesh::group::MeshDrawGroup;
use std::rc::Rc;
use webgpu::util::DeviceExt;

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
    self.mesh_gpu.setup_pass(ctx);
    ctx.set_vertex_buffer_owned_next(&self.instance_gpu);
  }
}

impl<M: WebGPUMesh> WebGPUMesh for TransformInstance<M> {
  type GPU = TransformInstanceGPU<M>;

  fn update(&self, gpu_mesh: &mut Self::GPU, gpu: &webgpu::GPU, storage: &mut anymap::AnyMap) {
    *gpu_mesh = self.create(gpu, storage)
  }

  fn create(&self, gpu: &webgpu::GPU, storage: &mut anymap::AnyMap) -> Self::GPU {
    let mesh_gpu = self.mesh.create(gpu, storage);
    let instance_gpu = gpu
      .device
      .deref()
      .create_buffer_init(&webgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(self.transforms.as_slice()),
        usage: BufferUsages::VERTEX,
      });
    TransformInstanceGPU {
      mesh_gpu,
      instance_gpu: Rc::new(instance_gpu),
    }
  }

  fn draw_impl(&self, group: MeshDrawGroup) -> DrawCommand {
    let mut inner = self.mesh.draw_impl(group);
    match &mut inner {
      DrawCommand::Indexed { instances, .. } => {
        assert_eq!(*instances, 0..1);
        *instances = 0..self.transforms.len() as u32;
      }
      DrawCommand::Array { instances, .. } => {
        assert_eq!(*instances, 0..1);
        *instances = 0..self.transforms.len() as u32;
      }
    }
    inner
  }

  fn topology(&self) -> webgpu::PrimitiveTopology {
    self.mesh.topology()
  }

  fn try_pick(
    &self,
    _f: &mut dyn FnMut(&dyn rendiation_renderable_mesh::mesh::IntersectAbleGroupedMesh),
  ) {
    // todo support picking?
  }
}
