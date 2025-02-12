use rendiation_shader_library::shader_uv_space_to_world_space;
use rendiation_texture_gpu_process::*;

use crate::*;

pub struct ViewerOutlineSourceProvider<'a> {
  pub g_buffer: &'a FrameGeometryBuffer,
  pub reproject: &'a UniformBufferCachedDataView<ReprojectInfo>,
}

impl ShaderHashProvider for ViewerOutlineSourceProvider<'_> {
  shader_hash_type_id! {ViewerOutlineSourceProvider<'static>}
}

impl OutlineComputeSourceProvider for ViewerOutlineSourceProvider<'_> {
  fn build(&self, binding: &mut ShaderBindGroupBuilder) -> Box<dyn OutlineComputeSourceInvocation> {
    let g_buffer = self.g_buffer.build_read_invocation(binding);
    let reproject = binding.bind_by(self.reproject).load().expand();
    Box::new(ViewerOutlineSourceInvocation {
      g_buffer,
      reproject,
    })
  }

  fn bind(&self, cx: &mut GPURenderPassCtx) {
    self.g_buffer.setup_pass(cx);
    cx.binding.bind(self.reproject);
  }
}

struct ViewerOutlineSourceInvocation {
  g_buffer: FrameGeometryBufferReadInvocation,
  reproject: ReprojectInfoShaderAPIInstance,
}

impl OutlineComputeSourceInvocation for ViewerOutlineSourceInvocation {
  fn get_source(&self, uv: Node<Vec2<f32>>) -> OutlineSource {
    let (depth, normal) = self.g_buffer.read_depth_normal(uv);
    let position_world =
      shader_uv_space_to_world_space(self.reproject.current_camera_view_projection_inv, uv, depth);

    let id = self.g_buffer.read_id(uv);

    OutlineSource {
      position: position_world,
      normal,
      entity_id: id,
    }
  }
}
