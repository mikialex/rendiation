use std::{
  any::{Any, TypeId},
  hash::Hash,
  ops::{Deref, DerefMut},
  rc::Rc,
};
pub mod states;
pub use states::*;

pub mod wrapper;
pub use wrapper::*;

// pub mod flat;
// pub use flat::*;
// pub mod line;
// pub use line::*;
pub mod physical;
pub use physical::*;
// pub mod fatline;
// pub use fatline::*;
// pub mod env_background;
// pub use env_background::*;

use rendiation_webgpu::*;

use crate::*;

pub trait MaterialMeshLayoutRequire {
  type VertexInput;
}

pub trait ShaderBindingProvider {
  fn setup_binding(&self, builder: &mut BindingBuilder);
}

pub trait ShaderHashProvider {
  fn hash_pipeline(&self, _hasher: &mut PipelineHasher) {}
}

pub trait RenderPassBuilder {
  fn setup_pass<'a>(&self, pass: GPURenderPass<'a>);
}

impl<T: ShaderBindingProvider> RenderPassBuilder for T {
  fn setup_pass<'a>(&self, pass: GPURenderPass<'a>) {
    todo!()
  }
}

pub trait SourceOfRendering:
  ShaderHashProvider // able to get pipeline from cache at low cost
   + ShaderGraphProvider // able to provide shader logic and config pipeline
   + RenderPassBuilder // able to bind resource to renderpass
{
}

impl<T> SourceOfRendering for T where T: ShaderHashProvider + ShaderGraphProvider + RenderPassBuilder
{}

#[derive(Default)]
pub struct RenderSourceBuilder<'a> {
  source: Vec<&'a dyn SourceOfRendering>,
}

impl<'a> RenderSourceBuilder<'a> {
  pub fn setup_pass(&self) {
    //
  }
}

pub trait WebGPUMaterial: Clone + Any {
  type GPU: SourceOfRendering;
  fn create_gpu(&self, res: &mut GPUResourceSubCache) -> Self::GPU;
  fn is_keep_mesh_shape(&self) -> bool;
  fn is_transparent(&self) -> bool;
}

type MaterialResourceMapper<T> = ResourceMapper<<T as WebGPUMaterial>::GPU, T>;

impl GPUResourceSceneCache {
  pub fn update_material<M: WebGPUMaterial>(
    &mut self,
    m: &ResourceWrapped<M>,
    gpu: &GPU,
    res: &mut GPUResourceSubCache,
  ) {
    let type_id = TypeId::of::<M>();

    let mapper = self
      .materials
      .entry(type_id)
      .or_insert_with(|| Box::new(MaterialResourceMapper::<M>::default()))
      .downcast_mut::<MaterialResourceMapper<M>>()
      .unwrap();

    let gpu_m = mapper.get_update_or_insert_with_logic(m, |x| match x {
      ResourceLogic::Create(m) => ResourceLogicResult::Create(M::create_gpu(m, res)),
      ResourceLogic::Update(gpu_m, m) => {
        // todo check should really recreate?
        *gpu_m = M::create_gpu(m, res);
        ResourceLogicResult::Update(gpu_m)
      }
    });

    // let m_gpu = gpu_m.gpu.as_mut().unwrap();

    // let topology = ctx.active_mesh.unwrap().topology();
    // let sample_count = ctx.pass_info.format_info.sample_count;

    // let mut hasher = Default::default();

    // type_id.hash(&mut hasher);
    // ctx.pass_info.format_info.hash(&mut hasher);

    // let (pipelines, pipeline_ctx) = ctx.pipeline_ctx();

    // pipeline_ctx.pass.type_id().hash(&mut hasher);
    // m.hash_pipeline(&mut hasher, &m_gpu);

    // gpu_m.current_pipeline = pipelines
    //   .get_or_insert_with(hasher, || {
    //     build_pipeline(
    //       &[
    //         ctx.pass as &dyn ShaderGraphProvider,
    //         m_gpu as &dyn ShaderGraphProvider,
    //       ]
    //       .as_slice(),
    //       &gpu.device,
    //     )
    //     .unwrap()

    //     // let mut builder = PipelineBuilder::default();

    //     // builder.primitive_state.topology = topology;
    //     // builder.multisample.count = sample_count;

    //     // m_gpu.create_pipeline(m, &mut builder, &gpu.device, &pipeline_ctx);
    //     // pipeline_ctx.pass.build_pipeline(&mut builder);
    //     // builder.build(&gpu.device)
    //   })
    //   .clone()
    //   .into();

    // let mut binding_builder = BindGroupBuilder::create();
    // m_gpu.setup_binding(&mut binding_builder);
    // // gpu_m.current_pipeline =
    // // binding_builder.
  }

  // pub fn setup_material<'a, M: WebGPUMaterial>(
  //   &self,
  //   m: &ResourceWrapped<M>,
  //   pass: &mut GPURenderPass<'a>,
  //   ctx: &SceneMaterialPassSetupCtx,
  // ) {
  //   let type_id = TypeId::of::<M>();
  //   let gpu_m = self
  //     .materials
  //     .get(&type_id)
  //     .unwrap()
  //     .downcast_ref::<MaterialResourceMapper<M>>()
  //     .unwrap()
  //     .get_unwrap(m);
  //   let gpu = gpu_m.gpu.as_ref().unwrap();

  //   pass.set_pipeline_owned(gpu_m.current_pipeline.as_ref().unwrap());

  //   // gpu.setup_pass_bindgroup(pass, ctx)
  //   todo!()
  // }
}

pub struct SceneMaterialRenderPrepareCtx<'a, 'b> {
  pub active_mesh: Option<&'b dyn WebGPUMesh>,
  pub base: &'b mut SceneMaterialRenderPrepareCtxBase<'a>,
}

impl<'a, 'b> Deref for SceneMaterialRenderPrepareCtx<'a, 'b> {
  type Target = SceneMaterialRenderPrepareCtxBase<'a>;

  fn deref(&self) -> &Self::Target {
    self.base
  }
}

impl<'a, 'b> DerefMut for SceneMaterialRenderPrepareCtx<'a, 'b> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.base
  }
}

pub trait PassDispatcher: Any + SourceOfRendering {}

pub struct DefaultPassDispatcher;

impl ShaderGraphProvider for DefaultPassDispatcher {
  fn build_vertex(
    &self,
    builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder
      .bindgroups
      .register_uniform::<UniformBuffer<RenderPassGPUInfoData>>(SB::Pass);
    Ok(())
  }
}

pub struct SceneMaterialRenderPrepareCtxBase<'a> {
  pub camera: &'a SceneCamera,
  pub pass_info: &'a RenderPassInfo,
  pub pass: &'a dyn PassDispatcher,
  pub resources: &'a mut GPUResourceSubCache,
}

pub struct SceneMaterialPassSetupCtx<'a> {
  pub resources: &'a GPUResourceSubCache,
  pub model_gpu: Option<&'a TransformGPU>,
  pub camera_gpu: &'a CameraGPUStore,
}
