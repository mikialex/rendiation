pub mod states;
pub use states::*;
pub mod flat;
pub use flat::*;
pub mod physical;
pub use physical::*;
pub mod fatline;
pub use fatline::*;

use crate::*;

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
