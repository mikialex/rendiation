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
pub mod semantic;
pub use semantic::*;

pub mod binding;
pub use binding::*;

pub mod flat;
pub use flat::*;
pub mod line;
pub use line::*;
pub mod physical;
pub use physical::*;
pub mod fatline;
pub use fatline::*;
pub mod env_background;
pub use env_background::*;

use rendiation_webgpu::{
  BindGroupLayoutCache, GPURenderPass, PipelineBuilder, PipelineHasher, PipelineResourceCache,
  RenderPassInfo, GPU,
};

use crate::*;

pub trait MaterialMeshLayoutRequire {
  type VertexInput;
}

pub trait MaterialCPUResource: Clone + Any {
  type GPU: MaterialGPUResource<Source = Self>;
  fn update_gpu(&self, ctx: &mut SceneMaterialRenderPrepareCtx) -> Self::GPU;
  fn is_keep_mesh_shape(&self) -> bool;
  fn is_transparent(&self) -> bool;
}

pub trait MaterialGPUResource: Sized + ShaderGraphProvider {
  type Source: MaterialCPUResource<GPU = Self>;

  fn hash_pipeline(&self, _source: &Self::Source, _hasher: &mut PipelineHasher) {}
}

type MaterialResourceMapper<T> = ResourceMapper<MaterialWebGPUResource<T>, T>;

impl GPUResourceSceneCache {
  pub fn update_material<M: MaterialCPUResource>(
    &mut self,
    m: &ResourceWrapped<M>,
    gpu: &GPU,
    ctx: &mut SceneMaterialRenderPrepareCtx,
  ) {
    let type_id = TypeId::of::<M>();

    let mapper = self
      .materials
      .entry(type_id)
      .or_insert_with(|| Box::new(MaterialResourceMapper::<M>::default()))
      .downcast_mut::<MaterialResourceMapper<M>>()
      .unwrap();

    let gpu_m = mapper.get_update_or_insert_with_logic(m, |x| match x {
      ResourceLogic::Create(m) => {
        let mut gpu_m = MaterialWebGPUResource::<M>::default();
        gpu_m.gpu = M::create(m, gpu, ctx, &gpu_m.bindgroup_watcher).into();
        ResourceLogicResult::Create(gpu_m)
      }
      ResourceLogic::Update(gpu_m, m) => {
        if gpu_m
          .gpu
          .as_mut()
          .unwrap()
          .update(m, gpu, ctx, &gpu_m.bindgroup_watcher)
        {
          gpu_m
            .gpu
            .replace(M::create(m, gpu, ctx, &gpu_m.bindgroup_watcher));
        }

        gpu_m.refresh_cache();

        ResourceLogicResult::Update(gpu_m)
      }
    });

    let m_gpu = gpu_m.gpu.as_mut().unwrap();

    let topology = ctx.active_mesh.unwrap().topology();
    let sample_count = ctx.pass_info.format_info.sample_count;

    let mut hasher = Default::default();

    type_id.hash(&mut hasher);
    ctx.pass_info.format_info.hash(&mut hasher);

    let (pipelines, pipeline_ctx) = ctx.pipeline_ctx();

    pipeline_ctx.pass.type_id().hash(&mut hasher);
    m_gpu.hash_pipeline(m, &mut hasher);

    gpu_m.current_pipeline = pipelines
      .get_or_insert_with(hasher, || {
        build_shader(
          &[
            ctx.pass as &dyn ShaderGraphProvider,
            m_gpu as &dyn ShaderGraphProvider,
          ]
          .as_slice(),
          &WGSL,
        );

        // let mut builder = PipelineBuilder::default();

        // builder.primitive_state.topology = topology;
        // builder.multisample.count = sample_count;

        // m_gpu.create_pipeline(m, &mut builder, &gpu.device, &pipeline_ctx);
        // pipeline_ctx.pass.build_pipeline(&mut builder);
        // builder.build(&gpu.device)

        todo!()
      })
      .clone()
      .into();
  }

  pub fn setup_material<'a, M: MaterialCPUResource>(
    &self,
    m: &ResourceWrapped<M>,
    pass: &mut GPURenderPass<'a>,
    ctx: &SceneMaterialPassSetupCtx,
  ) {
    let type_id = TypeId::of::<M>();
    let gpu_m = self
      .materials
      .get(&type_id)
      .unwrap()
      .downcast_ref::<MaterialResourceMapper<M>>()
      .unwrap()
      .get_unwrap(m);
    let gpu = gpu_m.gpu.as_ref().unwrap();

    pass.set_pipeline_owned(gpu_m.current_pipeline.as_ref().unwrap());

    gpu.setup_pass_bindgroup(pass, ctx)
  }
}

pub struct MaterialWebGPUResource<T: MaterialCPUResource> {
  current_pipeline: Option<Rc<wgpu::RenderPipeline>>,
  gpu: Option<T::GPU>,
}

impl<T: MaterialCPUResource> Default for MaterialWebGPUResource<T> {
  fn default() -> Self {
    Self {
      current_pipeline: Default::default(),
      gpu: Default::default(),
    }
  }
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

pub trait PassDispatcher: Any + ShaderGraphProvider {
  fn build_pipeline(&self, builder: &mut PipelineBuilder);
}

pub struct DefaultPassDispatcher;

impl ShaderGraphProvider for DefaultPassDispatcher {
  fn build_vertex(
    &self,
    builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder
      .bindgroups
      .register_uniform::<RenderPassGPUInfoData>();
    Ok(())
  }
}
impl PassDispatcher for DefaultPassDispatcher {
  fn build_pipeline(&self, _builder: &mut PipelineBuilder) {}
}

pub struct SceneMaterialRenderPrepareCtxBase<'a> {
  pub camera: &'a SceneCamera,
  pub pass_info: &'a RenderPassInfo,
  pub pass: &'a dyn PassDispatcher,
  pub resources: &'a mut GPUResourceSubCache,
}

impl<'a, 'b> SceneMaterialRenderPrepareCtx<'a, 'b> {
  pub fn pipeline_ctx(&mut self) -> (&mut PipelineResourceCache, PipelineCreateCtx) {
    (
      &mut self.base.resources.pipeline_resource,
      PipelineCreateCtx {
        layouts: &self.base.resources.layouts,
        active_mesh: self.active_mesh,
        pass_info: self.base.pass_info,
        pass: self.base.pass,
      },
    )
  }
}

pub struct PipelineCreateCtx<'a, 'b> {
  pub layouts: &'a BindGroupLayoutCache,
  pub active_mesh: Option<&'a dyn WebGPUMesh>,
  pub pass_info: &'b RenderPassInfo,
  pub pass: &'b dyn PassDispatcher,
}

pub struct SceneMaterialPassSetupCtx<'a> {
  pub resources: &'a GPUResourceSubCache,
  pub model_gpu: Option<&'a TransformGPU>,
  pub camera_gpu: &'a CameraBindgroup,
}
