use rendiation_shader_library::octahedral::decode_octahedral_normal_fn;

use crate::*;

#[derive(Clone)]
pub struct AttributeMeshIndirectDispatcher {
  pub sm_to_mesh: AbstractReadonlyStorageBuffer<[u32]>,
  pub vertex_address_buffer: AbstractReadonlyStorageBuffer<[AttributeMeshMeta]>,
  pub index_pool: AbstractReadonlyStorageBuffer<[u32]>,
  pub vertices: AbstractReadonlyStorageBuffer<[u32]>,
  pub enable_normal_quantization: bool,
}

impl ShaderHashProvider for AttributeMeshIndirectDispatcher {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    hasher.hash(self.enable_normal_quantization);
  }
}

#[derive(Clone)]
pub struct AttributeMeshIndirectRasterDispatcher {
  pub internal: AttributeMeshIndirectDispatcher,
  pub indices_ty: Option<IndexFormat>,
  pub topology: rendiation_webgpu::PrimitiveTopology,
}

impl ShaderHashProvider for AttributeMeshIndirectRasterDispatcher {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.internal.hash_pipeline(hasher);
    hasher.hash(self.indices_ty);
    hasher.hash(self.topology);
  }
}

impl ShaderPassBuilder for AttributeMeshIndirectRasterDispatcher {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    let mesh = &self.internal;

    if let Some(fmt) = self.indices_ty {
      // may be failed if we are using texture as storage
      if let Some(index) = mesh.index_pool.get_gpu_buffer_view() {
        ctx
          .pass
          .set_index_buffer_by_buffer_resource_view(&index, fmt);
      }
    }

    mesh.bind_base_invocation(&mut ctx.binding);
  }
}

impl GraphicsShaderProvider for AttributeMeshIndirectRasterDispatcher {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|vertex, binding| {
      let mesh_handle = vertex.query::<IndirectAbstractMeshId>();

      let vertex_id = vertex.query::<VertexIndex>();

      let mesh_sys = self.internal.build_base_invocation(binding);
      let (position, normal, uv) = mesh_sys.get_position_normal_uv(mesh_handle, vertex_id);

      vertex.register::<GeometryPosition>(position);
      vertex.register::<GeometryNormal>(normal);
      vertex.register::<GeometryUV>(uv);

      vertex.primitive_state().topology = self.topology;
    })
  }
}

#[derive(Clone)]
pub struct IndirectAttributeMeshDispatcherBaseInvocation {
  pub vertex_address_buffer: ShaderReadonlyPtrOf<[AttributeMeshMeta]>,
  pub vertices: ShaderReadonlyPtrOf<[u32]>,
  pub enable_normal_quantization: bool,
}

impl IndirectAttributeMeshDispatcherBaseInvocation {
  pub fn get_position_normal_uv(
    &self,
    mesh_handle: Node<u32>,
    vertex_id: Node<u32>,
  ) -> (Node<Vec3<f32>>, Node<Vec3<f32>>, Node<Vec2<f32>>) {
    let position = self.get_position(mesh_handle, vertex_id);
    let normal = self.get_normal(mesh_handle, vertex_id);
    let uv = self.get_uv(mesh_handle, vertex_id);
    (position, normal, uv)
  }

  pub fn get_normal(&self, mesh_handle: Node<u32>, vertex_id: Node<u32>) -> Node<Vec3<f32>> {
    let meta = self.vertex_address_buffer.index(mesh_handle);
    let normal_offset = meta.normal_offset().load();

    normal_offset.equals(u32::MAX).select_branched(
      || val(Vec3::zero()),
      || {
        if self.enable_normal_quantization {
          decode_octahedral_normal_fn(self.vertices.index(normal_offset + vertex_id).load())
        } else {
          let layout = StructLayoutTarget::Packed;
          unsafe {
            Vec3::<f32>::sized_ty()
              .load_from_u32_buffer(&self.vertices, normal_offset + vertex_id * val(3), layout)
              .into_node::<Vec3<f32>>()
          }
        }
      },
    )
  }

  pub fn get_position(&self, mesh_handle: Node<u32>, vertex_id: Node<u32>) -> Node<Vec3<f32>> {
    let meta = self.vertex_address_buffer.index(mesh_handle);
    let position_offset = meta.position_offset().load();
    // todo assert position_offset != u32::MAX
    let layout = StructLayoutTarget::Packed;
    unsafe {
      Vec3::<f32>::sized_ty()
        .load_from_u32_buffer(&self.vertices, position_offset + vertex_id * val(3), layout)
        .into_node::<Vec3<f32>>()
    }
  }

  pub fn get_uv(&self, mesh_handle: Node<u32>, vertex_id: Node<u32>) -> Node<Vec2<f32>> {
    let meta = self.vertex_address_buffer.index(mesh_handle);
    let uv_offset = meta.uv_offset().load();

    uv_offset.equals(u32::MAX).select_branched(
      || val(Vec2::zero()),
      || {
        let layout = StructLayoutTarget::Packed;
        unsafe {
          Vec2::<f32>::sized_ty()
            .load_from_u32_buffer(&self.vertices, uv_offset + vertex_id * val(2), layout)
            .into_node::<Vec2<f32>>()
        }
      },
    )
  }
}

impl AttributeMeshIndirectDispatcher {
  pub fn build_base_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> IndirectAttributeMeshDispatcherBaseInvocation {
    IndirectAttributeMeshDispatcherBaseInvocation {
      vertex_address_buffer: cx.bind_by(&self.vertex_address_buffer),
      vertices: cx.bind_by(&self.vertices),
      enable_normal_quantization: self.enable_normal_quantization,
    }
  }
  pub fn bind_base_invocation(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.vertex_address_buffer);
    cx.bind(&self.vertices);
  }
}
