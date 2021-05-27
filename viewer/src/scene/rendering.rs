use crate::renderer::RenderPassCreator;

use super::*;

impl RenderPassCreator<wgpu::SwapChainFrame> for Scene {
  fn create<'a>(
    &self,
    target: &'a wgpu::SwapChainFrame,
    encoder: &'a mut wgpu::CommandEncoder,
  ) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: "scene pass".into(),
      color_attachments: &[wgpu::RenderPassColorAttachment {
        view: &target.output.view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: self.get_main_pass_load_op(),
          store: true,
        },
      }],
      depth_stencil_attachment: None,
    })
  }
}

impl Scene {
  fn get_main_pass_load_op(&self) -> wgpu::LoadOp<wgpu::Color> {
    if let Some(clear_color) = self.background.require_pass_clear() {
      return wgpu::LoadOp::Clear(clear_color);
    }

    return wgpu::LoadOp::Load;
  }
}

pub struct RenderList {
  models: Vec<ModelHandle>,
}

impl Renderable for Scene {
  fn setup_pass<'a>(&'a mut self, pass: &mut wgpu::RenderPass<'a>) {
    let models = &self.models;
    let ctx = ModelPassSetupContext {
      materials: &self.materials,
      meshes: &self.meshes,
    };
    self
      .nodes
      .traverse_mut(self.get_root_handle(), &mut Vec::new(), |this, parent| {
        let node_data = this.data_mut();
        node_data.hierarchy_update(parent.map(|p| p.data()));
        node_data.payloads.iter().for_each(|payload| match payload {
          SceneNodePayload::Model(model) => {
            let model = models.get(*model).unwrap();
            model.setup_pass(pass, &ctx)
          }
          _ => {}
        });
        if node_data.net_visible {
          NextTraverseVisit::SkipChildren
        } else {
          NextTraverseVisit::VisitChildren
        }
      });
  }

  fn update(&mut self, renderer: &Renderer, encoder: &mut wgpu::CommandEncoder) {
    let root = self.get_root_handle();
    let models = &mut self.models;
    let materials = &mut self.materials;
    let meshes = &mut self.meshes;
    let pipelines = &mut self.pipeline_resource;
    self
      .nodes
      .traverse_mut(root, &mut Vec::new(), |this, parent| {
        let node_data = this.data_mut();
        node_data.hierarchy_update(parent.map(|p| p.data()));

        let mut ctx = ModelPassPrepareContext {
          materials,
          meshes,
          material_ctx: SceneMaterialRenderPrepareCtx {
            camera: todo!(),
            camera_gpu: todo!(),
            model_matrix: todo!(),
            model_matrix_gpu: todo!(),
            pipelines,
          },
        };

        node_data.payloads.iter().for_each(|payload| match payload {
          SceneNodePayload::Model(model) => {
            let model = models.get_mut(*model).unwrap();
            model.update(&mut ctx, renderer)
          }
          _ => {}
        });

        if node_data.net_visible {
          NextTraverseVisit::SkipChildren
        } else {
          NextTraverseVisit::VisitChildren
        }
      });
  }
}
