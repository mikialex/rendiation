use rendiation_shader_library::shader_uv_space_to_world_space;
use rendiation_texture_gpu_base::SamplerConvertExt;
use rendiation_texture_gpu_process::*;

use crate::*;

pub struct ViewerOutlineSourceProvider<'a> {
  pub depth: &'a Attachment,
  pub ids: &'a Attachment,
  pub reproject: &'a UniformBufferCachedDataView<ReprojectInfo>,
}

impl ShaderHashProvider for ViewerOutlineSourceProvider<'_> {
  shader_hash_type_id! {ViewerOutlineSourceProvider<'static>}
}

impl OutlineComputeSourceProvider for ViewerOutlineSourceProvider<'_> {
  fn build(&self, binding: &mut ShaderBindGroupBuilder) -> Box<dyn OutlineComputeSourceInvocation> {
    let depth = binding.bind_by(&DisableFiltering(&self.depth.read()));
    let ids = binding.bind_by(&DisableFiltering(&self.ids.read()));
    let sampler = binding.bind_by(&DisableFiltering(ImmediateGPUSamplerViewBind));
    let reproject = binding.bind_by(self.reproject).load().expand();

    Box::new(ViewerOutlineSourceInvocation {
      depth,
      ids,
      sampler,
      reproject,
    })
  }

  fn bind(&self, cx: &mut GPURenderPassCtx) {
    self.depth.read().bind_pass(cx);
    self.ids.read().bind_pass(cx);
    cx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    cx.binding.bind(self.reproject);
  }
}

struct ViewerOutlineSourceInvocation {
  depth: HandleNode<ShaderTexture2D>,
  ids: HandleNode<ShaderTexture2D>,
  sampler: HandleNode<ShaderSampler>,
  reproject: ReprojectInfoShaderAPIInstance,
}

impl OutlineComputeSourceInvocation for ViewerOutlineSourceInvocation {
  fn get_source(&self, uv: Node<Vec2<f32>>) -> OutlineSource {
    let depth = self.depth.sample(self.sampler, uv).x();
    let position_world =
      shader_uv_space_to_world_space(self.reproject.current_camera_view_projection_inv, uv, depth);

    let id = self.ids.sample(self.sampler, uv).x();

    OutlineSource {
      position: position_world,
      normal: val(Vec3::zero()),
      entity_id: id.into_u32(), // todo check if sample is correct
    }
  }
}
