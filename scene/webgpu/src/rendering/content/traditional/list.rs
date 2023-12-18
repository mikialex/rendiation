use core::cmp::Ordering::Equal;
use std::sync::RwLockReadGuard;

use crate::*;

#[derive(Default)]
pub struct RenderList {
  pub(crate) opaque: Vec<(SceneModel, f32)>,
  pub(crate) transparent: Vec<(SceneModel, f32)>,
}

impl RenderList {
  pub fn collect_from_scene_objects(
    &mut self,
    scene: &SceneRenderResourceGroup,
    iter: impl Iterator<Item = SceneModel>,
    camera: &SceneCamera,
    blend: bool,
  ) {
    if scene.scene.active_camera.is_none() {
      return;
    }

    let camera_mat = camera.visit(|camera| scene.node_derives.get_world_matrix(&camera.node));
    let camera_pos = camera_mat.position();
    let camera_forward = camera_mat.forward().reverse();

    self.opaque.clear();
    self.transparent.clear();

    for m in iter {
      let model_pos = scene
        .node_derives
        .get_world_matrix(&m.read().node)
        .position();
      let depth = (model_pos - camera_pos).dot(camera_forward);

      if blend && m.read().model.should_use_alpha_blend() {
        self.transparent.push((m.clone(), depth));
      } else {
        self.opaque.push((m.clone(), depth));
      }
    }

    self
      .opaque
      .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Equal));
    self
      .transparent
      .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Equal));
  }
  pub fn collect_from_scene(
    &mut self,
    scene: &SceneRenderResourceGroup,
    camera: &SceneCamera,
    blend: bool,
  ) {
    self.collect_from_scene_objects(
      scene,
      scene.scene.models.iter().map(|(_, m)| m.clone()),
      camera,
      blend,
    )
  }

  pub fn setup_pass(
    &self,
    gpu_pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    resource: &SceneRenderResourceGroup,
    skip_opaque: bool,
  ) {
    let resource_view = ModelGPURenderResourceView::new(resource);
    let camera_gpu = resource_view.cameras.get_camera_gpu(camera).unwrap();

    if !skip_opaque {
      self.opaque.iter().for_each(|(model, _)| {
        scene_model_setup_pass_core(
          gpu_pass,
          model.guid(),
          camera_gpu,
          &resource_view,
          dispatcher,
        );
      });
    }

    self.transparent.iter().for_each(|(model, _)| {
      scene_model_setup_pass_core(
        gpu_pass,
        model.guid(),
        camera_gpu,
        &resource_view,
        dispatcher,
      );
    });
  }
}

pub(crate) struct ModelGPURenderResourceView<'a> {
  pub(crate) nodes: &'a SceneNodeGPUSystem,
  pub(crate) cameras: RwLockReadGuard<'a, SceneCameraGPUSystem>,
  pub(crate) scene_models: RwLockReadGuard<'a, StreamMap<u64, ReactiveSceneModelGPUInstance>>,
  pub(crate) models: RwLockReadGuard<'a, StreamMap<u64, ReactiveModelGPUType>>,
  pub(crate) materials: RwLockReadGuard<'a, StreamMap<u64, MaterialGPUInstance>>,
  pub(crate) meshes: RwLockReadGuard<'a, StreamMap<u64, MeshGPUInstance>>,
}

impl<'a> ModelGPURenderResourceView<'a> {
  pub fn new(pass: &SceneRenderResourceGroup<'a>) -> Self {
    Self {
      nodes: &pass.scene_resources.nodes,
      cameras: pass.scene_resources.cameras.read().unwrap(),
      scene_models: pass.scene_resources.models.read().unwrap(),
      models: pass.resources.models.read().unwrap(),
      materials: pass.resources.model_ctx.materials.read().unwrap(),
      meshes: pass.resources.model_ctx.meshes.read().unwrap(),
    }
  }
}

pub(crate) fn scene_model_setup_pass_core(
  gpu_pass: &mut FrameRenderPass,
  model_guid: u64,
  camera_gpu: &CameraGPU,
  resource_view: &ModelGPURenderResourceView,
  dispatcher: &dyn RenderComponentAny,
) {
  let scene_model = resource_view.scene_models.get(&model_guid).unwrap();
  let scene_model = scene_model.as_ref();

  let model_id = scene_model.model_id.unwrap();
  let model_gpu = resource_view.models.get(&model_id).unwrap();
  let node_gpu = resource_view.nodes.get_by_raw(scene_model.node_id).unwrap();

  if let ReactiveModelGPUType::Standard(m_gpu) = model_gpu {
    model_setup_pass_core(
      gpu_pass,
      m_gpu.as_ref(),
      camera_gpu,
      node_gpu,
      resource_view,
      dispatcher,
    );
  }
}

fn model_setup_pass_core(
  pass: &mut FrameRenderPass,
  model_gpu: &StandardModelGPU,
  camera_gpu: &CameraGPU,
  node_gpu: &NodeGPU,
  ctx: &ModelGPURenderResourceView,
  dispatcher: &dyn RenderComponentAny,
) {
  let material_gpu = ctx.materials.get(&model_gpu.material_id.unwrap()).unwrap();
  let mesh_gpu = ctx.meshes.get(&model_gpu.mesh_id.unwrap()).unwrap();
  let pass_gpu = dispatcher;

  let draw_command = mesh_gpu.draw_command(model_gpu.group);

  dispatch_model_draw_with_preferred_binding_frequency(
    pass_gpu,
    mesh_gpu,
    node_gpu,
    camera_gpu,
    material_gpu,
    draw_command,
    &mut pass.ctx,
  )
}

pub trait AlphaBlendDecider {
  fn should_use_alpha_blend(&self) -> bool;
}
define_dyn_trait_downcaster_static!(AlphaBlendDecider);

impl AlphaBlendDecider for IncrementalSignalPtr<StandardModel> {
  fn should_use_alpha_blend(&self) -> bool {
    self.read().material.is_transparent()
  }
}

impl AlphaBlendDecider for ModelEnum {
  fn should_use_alpha_blend(&self) -> bool {
    match self {
      ModelEnum::Standard(model) => model.should_use_alpha_blend(),
      ModelEnum::Foreign(any) => {
        if let Some(any) =
          get_dyn_trait_downcaster_static!(AlphaBlendDecider).downcast_ref(any.as_ref().as_any())
        {
          any.should_use_alpha_blend()
        } else {
          false
        }
      }
    }
  }
}
