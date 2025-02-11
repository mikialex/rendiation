use rendiation_shader_library::shader_uv_space_to_world_space;
use rendiation_texture_gpu_base::SamplerConvertExt;
use rendiation_texture_gpu_process::*;

use crate::*;

pub struct ViewerOutlineSourceProvider<'a> {
  pub g_buffer: &'a FrameGeometryBuffer,
  pub reproject: &'a UniformBufferCachedDataView<ReprojectInfo>,
}

impl ShaderHashProvider for ViewerOutlineSourceProvider<'_> {
  shader_hash_type_id! {ViewerOutlineSourceProvider<'static>}
}

// work around
struct U32Texture2d;
impl ShaderBindingProvider for U32Texture2d {
  type Node = ShaderHandlePtr<ShaderTexture2DUint>;
}

impl OutlineComputeSourceProvider for ViewerOutlineSourceProvider<'_> {
  fn build(&self, binding: &mut ShaderBindGroupBuilder) -> Box<dyn OutlineComputeSourceInvocation> {
    let normal = binding.bind_by(&self.g_buffer.normal.read());
    let depth = binding.bind_by(&DisableFiltering(&self.g_buffer.depth.read()));
    let ids = binding.bind_by(&U32Texture2d);
    let sampler = binding.bind_by(&DisableFiltering(ImmediateGPUSamplerViewBind));
    let reproject = binding.bind_by(self.reproject).load().expand();

    let input_size = depth.texture_dimension_2d(None).into_f32();

    Box::new(ViewerOutlineSourceInvocation {
      depth,
      normal,
      ids,
      sampler,
      reproject,
      input_size,
    })
  }

  fn bind(&self, cx: &mut GPURenderPassCtx) {
    self.g_buffer.normal.read().bind_pass(cx);
    self.g_buffer.depth.read().bind_pass(cx);
    self.g_buffer.entity_id.read().bind_pass(cx);
    cx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    cx.binding.bind(self.reproject);
  }
}

struct ViewerOutlineSourceInvocation {
  depth: HandleNode<ShaderTexture2D>,
  normal: HandleNode<ShaderTexture2D>,
  ids: HandleNode<ShaderTexture2DUint>,
  sampler: HandleNode<ShaderSampler>,
  reproject: ReprojectInfoShaderAPIInstance,
  input_size: Node<Vec2<f32>>,
}

impl OutlineComputeSourceInvocation for ViewerOutlineSourceInvocation {
  fn get_source(&self, uv: Node<Vec2<f32>>) -> OutlineSource {
    let depth = self.depth.sample(self.sampler, uv).x();
    let position_world =
      shader_uv_space_to_world_space(self.reproject.current_camera_view_projection_inv, uv, depth);

    let normal = self.normal.sample(self.sampler, uv).xyz().normalize();

    let u32_uv = (self.input_size * uv).floor().into_u32();
    let id = self.ids.load_texel(u32_uv, val(0)).x();

    OutlineSource {
      position: position_world,
      normal,
      entity_id: id,
    }
  }
}
