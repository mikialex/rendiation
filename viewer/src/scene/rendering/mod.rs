use rendiation_webgpu::*;

use super::*;
pub mod forward;
pub use forward::*;
// pub mod rg;
// pub use rg::*;

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
    self.pass.create_pass(self.scene, target, encoder)
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

    {
      scene.background.setup_pass(
        pass,
        &scene.materials,
        &scene.meshes,
        &scene.nodes,
        scene.active_camera_gpu.as_ref().unwrap(),
        &scene.pipeline_resource,
        self.pass,
      );
    }

    scene.render_list.models.iter().for_each(|model| {
      let model = models.get(*model).unwrap();
      model.setup_pass(
        pass,
        &scene.materials,
        &scene.meshes,
        &scene.nodes,
        scene.active_camera_gpu.as_ref().unwrap(),
        &scene.pipeline_resource,
        self.pass,
      )
    })
  }

  fn update(&mut self, gpu: &GPU, _encoder: &mut wgpu::CommandEncoder) {
    let scene = &mut self.scene;
    scene.render_list.models.clear();
    let root = scene.get_root_handle();

    scene.maintain(&gpu.device, &gpu.queue);

    {
      scene.get_root_node_mut().get_model_gpu(gpu);
    }

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

      let camera_gpu = scene
        .active_camera_gpu
        .get_or_insert_with(|| CameraBindgroup::new(gpu))
        .update(gpu, active_camera, &scene.nodes);

      let mut base = SceneMaterialRenderPrepareCtxBase {
        active_camera,
        camera_gpu,
        pass: self.pass,
        pipelines: &mut scene.pipeline_resource,
        layouts: &mut scene.layouts,
        textures: &mut scene.texture_2ds,
        texture_cubes: &mut scene.texture_cubes,
        samplers: &mut scene.samplers,
        reference_finalization: &scene.reference_finalization,
      };

      scene.background.update(
        gpu,
        &mut base,
        &mut scene.materials,
        &mut scene.meshes,
        &mut scene.nodes,
      );

      scene.models.iter_mut().for_each(|(handle, model)| {
        scene.render_list.models.push(handle);

        let material = scene.materials.get_mut(model.material()).unwrap().as_mut();
        let mesh = scene.meshes.get_mut(model.mesh()).unwrap();
        let node = scene.nodes.get_node_mut(model.node()).data_mut();

        let mut ctx = SceneMaterialRenderPrepareCtx {
          base: &mut base,
          model_info: node.get_model_gpu(gpu).into(),
          active_mesh: mesh.as_ref().into(),
        };

        material.update(gpu, &mut ctx);

        mesh.update(gpu);
      })
    }
  }
}
