use std::sync::RwLockReadGuard;

use crate::*;

#[derive(Default)]
pub struct RenderList {
  pub(crate) opaque: Vec<(SceneModelHandle, f32)>,
  pub(crate) transparent: Vec<(SceneModelHandle, f32)>,
}

pub fn is_model_enable_blend(model: &ModelType) -> bool {
  match model {
    ModelType::Standard(model) => model.read().material.is_transparent(),
    ModelType::Foreign(_) => false, // todo
    _ => false,
  }
}

impl RenderList {
  pub fn prepare(&mut self, scene: &SceneRenderResourceGroup, camera: &SceneCamera) {
    if scene.scene.active_camera.is_none() {
      return;
    }

    let camera_mat = camera.visit(|camera| scene.node_derives.get_world_matrix(&camera.node));
    let camera_pos = camera_mat.position();
    let camera_forward = camera_mat.forward().reverse();

    self.opaque.clear();
    self.transparent.clear();

    for (h, m) in scene.scene.models.iter() {
      let model_pos = scene
        .node_derives
        .get_world_matrix(&m.get_node())
        .position();
      let depth = (model_pos - camera_pos).dot(camera_forward);

      let is_transparent = is_model_enable_blend(&m.read().model);
      if is_transparent {
        self.transparent.push((h, depth));
      } else {
        self.opaque.push((h, depth));
      }
    }

    self.opaque.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    self
      .transparent
      .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
  }

  pub fn setup_pass(
    &self,
    gpu_pass: &mut FrameRenderPass,
    dispatcher: &dyn RenderComponentAny,
    camera: &SceneCamera,
    resource: &SceneRenderResourceGroup,
  ) {
    let resource_view = ModelGPURenderResourceView::new(resource);
    let camera_gpu = resource_view.cameras.get_camera_gpu(camera).unwrap();

    let models = &resource.scene.models;

    self.opaque.iter().for_each(|(handle, _)| {
      let model = models.get(*handle).unwrap();
      scene_model_setup_pass_core(
        gpu_pass,
        model.guid(),
        camera_gpu,
        &resource_view,
        dispatcher,
      );
    });
    self.transparent.iter().for_each(|(handle, _)| {
      let model = models.get(*handle).unwrap();
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

struct ModelGPURenderResourceView<'a> {
  nodes: &'a SceneNodeGPUSystem,
  cameras: RwLockReadGuard<'a, SceneCameraGPUSystem>,
  scene_models: RwLockReadGuard<'a, StreamMap<usize, ReactiveSceneModelGPUInstance>>,
  models: RwLockReadGuard<'a, StreamMap<usize, ReactiveModelGPUType>>,
  materials: RwLockReadGuard<'a, StreamMap<usize, MaterialGPUInstance>>,
  meshes: RwLockReadGuard<'a, StreamMap<usize, MeshGPUInstance>>,
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

fn scene_model_setup_pass_core(
  gpu_pass: &mut FrameRenderPass,
  model_guid: usize,
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
