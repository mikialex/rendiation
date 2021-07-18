use rendiation_webgpu::*;

use super::*;
pub mod forward;
pub use forward::*;
pub mod rg;
pub use rg::*;

pub trait ViewerRenderPass {
  fn depth_stencil_format(&self) -> Option<wgpu::TextureFormat>;
  fn color_format(&self) -> &[wgpu::TextureFormat];
}

pub trait ViewerRenderPassCreator {
  type TargetResource;

  fn create_pass<'a>(
    &'a self,
    scene: &Scene,
    target_res: &'a Self::TargetResource,
    encoder: &'a mut wgpu::CommandEncoder,
  ) -> wgpu::RenderPass<'a>;
}

impl<'b, S> RenderPassCreator<S::TargetResource> for RenderPassDispatcher<'b, S>
where
  S: ViewerRenderPassCreator,
{
  fn create<'a>(
    &'a self,
    target: &'a S::TargetResource,
    encoder: &'a mut wgpu::CommandEncoder,
  ) -> wgpu::RenderPass<'a> {
    self.pass.create_pass(&self.scene, target, encoder)
  }
}

pub struct RenderList {
  models: Vec<ModelHandle>,
}

impl RenderList {
  pub fn new() -> Self {
    Self { models: Vec::new() }
  }
}

pub struct RenderPassDispatcher<'a, S> {
  pub scene: &'a mut Scene,
  pub pass: &'a mut S,
}

impl<'a, S: ViewerRenderPassCreator + ViewerRenderPass> Renderable for RenderPassDispatcher<'a, S> {
  fn setup_pass<'p>(&'p self, pass: &mut wgpu::RenderPass<'p>) {
    let scene = &self.scene;
    let models = &scene.models;
    scene.render_list.models.iter().for_each(|model| {
      let model = models.get(*model).unwrap();
      let material = scene.materials.get(model.material()).unwrap().as_ref();
      let node = scene.nodes.get_node(model.node()).data();
      let mesh = scene.meshes.get(model.mesh()).unwrap();

      let ctx = SceneMaterialPassSetupCtx {
        pass: self.pass,
        camera_gpu: scene.active_camera_gpu.as_ref().unwrap(),
        model_gpu: node.gpu.as_ref().unwrap(),
        pipelines: &scene.pipeline_resource,
        active_mesh: mesh,
      };
      material.setup_pass(pass, &ctx);

      let mesh = scene.meshes.get(model.mesh()).unwrap();
      mesh.setup_pass(pass, model.group());
    })
  }

  fn update(&mut self, gpu: &mut GPU, encoder: &mut wgpu::CommandEncoder) {
    let scene = &mut self.scene;
    scene.render_list.models.clear();
    let root = scene.get_root_handle();

    scene.maintain(&gpu.device, &mut gpu.queue);

    if let Some(active_camera) = &mut scene.active_camera {
      scene
        .nodes
        .traverse_mut(root, &mut Vec::new(), |this, parent| {
          let node_data = this.data_mut();
          node_data.hierarchy_update(parent.map(|p| p.data()));
          if node_data.net_visible {
            NextTraverseVisit::SkipChildren
          } else {
            NextTraverseVisit::VisitChildren
          }
        });

      active_camera.update();
      let camera_gpu = scene
        .active_camera_gpu
        .get_or_insert_with(|| CameraBindgroup::new(gpu, active_camera))
        .update(gpu, active_camera, &scene.nodes);

      scene.models.iter_mut().for_each(|(handle, model)| {
        scene.render_list.models.push(handle);

        let material = scene.materials.get_mut(model.material()).unwrap().as_mut();
        let mesh = scene.meshes.get_mut(model.mesh()).unwrap();
        let node = scene.nodes.get_node_mut(model.node()).data_mut();
        let (model_matrix, model_gpu) = node.get_model_gpu(gpu);

        let mut ctx = SceneMaterialRenderPrepareCtx {
          active_camera,
          camera_gpu,
          model_matrix,
          model_gpu,
          pipelines: &mut scene.pipeline_resource,
          pass: self.pass,
          active_mesh: mesh,
          textures: &mut scene.texture_2ds,
          samplers: &mut scene.samplers,
          reference_finalization: &scene.reference_finalization,
        };
        material.update(gpu, &mut ctx);
        mesh.update(gpu);
      })
    }
  }
}
