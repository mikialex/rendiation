use std::{
  any::{Any, TypeId},
  ops::Deref,
};
pub mod states;
pub use states::*;

use __core::hash::Hash;
use rendiation_renderable_mesh::group::MeshDrawGroup;

pub mod flat;
pub use flat::*;
pub mod physical;
pub use physical::*;
pub mod fatline;
pub use fatline::*;

use webgpu::*;

use crate::*;

pub trait RenderComponent: ShaderHashProvider + ShaderGraphProvider + ShaderPassBuilder {
  fn render(&self, ctx: &mut GPURenderPassCtx, emitter: &dyn DrawcallEmitter) {
    let mut hasher = PipelineHasher::default();
    self.hash_pipeline(&mut hasher);

    let pipeline = ctx
      .gpu
      .device
      .get_or_cache_create_render_pipeline(hasher, |device| {
        device
          .build_pipeline_by_shadergraph(self.build_self().unwrap())
          .unwrap()
      });

    ctx.binding.reset();
    ctx.reset_vertex_binding_index();

    self.setup_pass(ctx);

    ctx.pass.set_pipeline_owned(&pipeline);

    ctx
      .binding
      .setup_pass(&mut ctx.pass, &ctx.gpu.device, &pipeline);

    emitter.draw(ctx);
  }
}

impl<T> RenderComponent for T where T: ShaderHashProvider + ShaderGraphProvider + ShaderPassBuilder {}

pub trait RenderComponentAny: RenderComponent + ShaderHashProviderAny {}
impl<T> RenderComponentAny for T where T: RenderComponent + ShaderHashProviderAny {}

pub trait DrawcallEmitter {
  fn draw(&self, ctx: &mut GPURenderPassCtx);
}

pub trait MeshDrawcallEmitter {
  fn draw(&self, ctx: &mut GPURenderPassCtx, group: MeshDrawGroup);
}

pub struct MeshDrawcallEmitterWrap<'a> {
  pub group: MeshDrawGroup,
  pub mesh: &'a dyn MeshDrawcallEmitter,
}

impl<'a> DrawcallEmitter for MeshDrawcallEmitterWrap<'a> {
  fn draw(&self, ctx: &mut GPURenderPassCtx) {
    self.mesh.draw(ctx, self.group)
  }
}

pub struct RenderEmitter<'a, 'b> {
  contents: &'a [&'b dyn RenderComponentAny],
}

impl<'a, 'b> RenderEmitter<'a, 'b> {
  pub fn new(contents: &'a [&'b dyn RenderComponentAny]) -> Self {
    Self { contents }
  }
}

impl<'a, 'b> ShaderPassBuilder for RenderEmitter<'a, 'b> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.contents.iter().for_each(|c| c.setup_pass(ctx));
  }
}

impl<'a, 'b> ShaderHashProvider for RenderEmitter<'a, 'b> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self
      .contents
      .iter()
      .for_each(|com| com.hash_pipeline_and_with_type_id(hasher))
  }
}

impl<'a, 'b> ShaderGraphProvider for RenderEmitter<'a, 'b> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    for c in self.contents {
      c.build(builder)?;
    }
    Ok(())
  }
}

pub trait WebGPUMaterial: Clone + Any {
  type GPU: RenderComponentAny;
  fn create_gpu(&self, res: &mut GPUResourceSubCache, gpu: &GPU) -> Self::GPU;
  fn is_keep_mesh_shape(&self) -> bool;
  fn is_transparent(&self) -> bool;
}

pub trait WebGPUSceneMaterial: 'static {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMaterialCache,
    sub_res: &mut GPUResourceSubCache,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny;
  fn is_keep_mesh_shape(&self) -> bool;
}

impl<M: WebGPUMaterial> WebGPUSceneMaterial for Identity<M> {
  fn check_update_gpu<'a>(
    &self,
    res: &'a mut GPUMaterialCache,
    sub_res: &mut GPUResourceSubCache,
    gpu: &GPU,
  ) -> &'a dyn RenderComponentAny {
    res.update_material(self, gpu, sub_res)
  }
  fn is_keep_mesh_shape(&self) -> bool {
    self.deref().is_keep_mesh_shape()
  }
}

type MaterialIdentityMapper<T> = IdentityMapper<<T as WebGPUMaterial>::GPU, T>;
impl GPUMaterialCache {
  pub fn update_material<M: WebGPUMaterial>(
    &mut self,
    m: &Identity<M>,
    gpu: &GPU,
    res: &mut GPUResourceSubCache,
  ) -> &M::GPU {
    let type_id = TypeId::of::<M>();

    let mapper = self
      .inner
      .entry(type_id)
      .or_insert_with(|| Box::new(MaterialIdentityMapper::<M>::default()))
      .downcast_mut::<MaterialIdentityMapper<M>>()
      .unwrap();

    mapper.get_update_or_insert_with_logic(m, |x| match x {
      ResourceLogic::Create(m) => ResourceLogicResult::Create(M::create_gpu(m, res, gpu)),
      ResourceLogic::Update(gpu_m, m) => {
        // todo check should really recreate?
        *gpu_m = M::create_gpu(m, res, gpu);
        ResourceLogicResult::Update(gpu_m)
      }
    })
  }
}

pub struct DefaultPassDispatcher {
  pub formats: RenderTargetFormatsInfo,
  pub pass_info: UniformBufferView<RenderPassGPUInfoData>,
}

impl ShaderHashProvider for DefaultPassDispatcher {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.formats.hash(hasher);
  }
}
impl ShaderPassBuilder for DefaultPassDispatcher {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.pass_info, SB::Pass);
  }
}

impl ShaderGraphProvider for DefaultPassDispatcher {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let pass = builder.bindgroups.uniform_by(&self.pass_info, SB::Pass);

    builder.vertex(|builder, _| {
      let pass = pass.using().expand();
      builder.register::<RenderBufferSize>(pass.buffer_size);
      Ok(())
    })?;
    builder.fragment(|builder, _| {
      let pass = pass.using().expand();
      builder.register::<RenderBufferSize>(pass.buffer_size);

      for &format in &self.formats.color_formats {
        builder.push_fragment_out_slot(ColorTargetState {
          format,
          blend: Some(webgpu::BlendState::ALPHA_BLENDING),
          write_mask: webgpu::ColorWrites::ALL,
        });
      }

      builder.depth_stencil = self
        .formats
        .depth_stencil_formats
        .map(|format| DepthStencilState {
          format,
          depth_write_enabled: true,
          depth_compare: CompareFunction::Always,
          stencil: Default::default(),
          bias: Default::default(),
        });

      builder.multisample.count = self.formats.sample_count;

      Ok(())
    })
  }
}
