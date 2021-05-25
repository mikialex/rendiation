pub mod background;
pub mod buffer;
pub mod lights;
pub mod node;
pub mod resource;

pub use background::*;
pub use buffer::*;
pub use lights::*;
pub use node::*;
pub use resource::*;

pub mod materials;
pub use materials::*;

pub use arena::*;
pub use arena_tree::*;

use crate::renderer::*;

impl RenderPassCreator<wgpu::SwapChainFrame> for Scene {
  fn create<'a>(
    &self,
    target: &'a wgpu::SwapChainFrame,
    encoder: &'a mut wgpu::CommandEncoder,
  ) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: "scene pass".into(),
      color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
        attachment: &target.output.view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Clear(wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
          }),
          store: true,
        },
      }],
      depth_stencil_attachment: None,
    })
  }
}

impl Renderable for Scene {
  fn render<'a>(
    &mut self,
    pass: &mut wgpu::RenderPass<'a>,
    // des: &wgpu::RenderPassDescriptor,
    res: &'a Self::Resource,
  ) {
    self.update();

    let root = self.get_root_handle();
    let nodes = &mut self.nodes;
    let models = &self.models;
    let mut ctx = SceneRenderCtx {
      materials: &mut self.materials,
      meshes: &mut self.meshes,
      material_ctx: SceneMaterialRenderPrepareCtx { camera: todo!() },
    };
    let mut model_list = Vec::new();
    nodes.traverse_mut(root, &mut Vec::new(), |node, _| {
      let node = node.data();
      node.payloads.iter().for_each(|payload| match payload {
        SceneNodePayload::Model(model) => {
          model_list.push(*model);
        }
        _ => {}
      });
      NextTraverseVisit::VisitChildren
    });
    model_list.iter().for_each(|model| {
      let model = models.get(*model).unwrap();
      model.render(pass, &mut ctx, res)
    })
  }

  type Resource = SceneResource;

  fn update(
    &mut self,
    renderer: &Renderer,
    res: &mut Self::Resource,
    encoder: &mut wgpu::CommandEncoder,
  ) {
    todo!()
  }
}

pub struct SceneMesh {
  vertex: Vec<VertexBuffer>,
  index: Option<IndexBuffer>,
}

impl SceneMesh {
  fn setup_pass<'a>(&mut self, pass: &mut wgpu::RenderPass<'a>, res: &'a SceneResource) {
    self.index.as_mut().map(|index| index.setup_pass(pass, res));
    self
      .vertex
      .iter_mut()
      .enumerate()
      .for_each(|(i, vertex)| vertex.setup_pass(pass, i as u32, res))
  }
}

pub struct SceneRenderCtx<'a> {
  materials: &'a mut Arena<Box<dyn Material>>,
  meshes: &'a mut Arena<SceneMesh>,
  material_ctx: SceneMaterialRenderPrepareCtx,
}

pub struct Model {
  material: MaterialHandle,
  mesh: MeshHandle,
}

impl Model {
  fn update(&mut self, ctx: &mut SceneRenderCtx, renderer: &mut Renderer, res: &mut SceneResource) {
    let material = ctx.materials.get_mut(self.material).unwrap();
    material.update(renderer, &mut ctx.material_ctx, res)
  }

  fn render<'a>(
    &self,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &mut SceneRenderCtx,
    res: &'a SceneResource,
  ) {
    let material = ctx.materials.get_mut(self.material).unwrap();
    // material.setup_pass(renderer, pass);
    let mesh = ctx.meshes.get_mut(self.mesh).unwrap();
    mesh.setup_pass(pass, res);
  }
}

use arena::{Arena, Handle};
use arena_tree::{ArenaTree, ArenaTreeNodeHandle, NextTraverseVisit};
use rendiation_texture::TextureSampler;

pub type SceneNodeHandle = ArenaTreeNodeHandle<SceneNode>;
pub type ModelHandle = Handle<Model>;
pub type MeshHandle = Handle<SceneMesh>;
pub type MaterialHandle = Handle<Box<dyn Material>>;
pub type LightHandle = Handle<Box<dyn Light>>;

pub struct Scene {
  pub nodes: ArenaTree<SceneNode>,
  pub background: Option<Box<dyn Background>>,
  pub lights: Arena<Box<dyn Light>>,
  pub models: Arena<Model>,
  pub meshes: Arena<SceneMesh>,
  pub materials: Arena<Box<dyn Material>>,
  pub samplers: Arena<TextureSampler>,
  // textures: Arena<Texture>,
  // buffers: Arena<Buffer>,
}

impl Scene {
  pub fn new() -> Self {
    Self {
      nodes: ArenaTree::new(SceneNode::default()),
      background: None,
      models: Arena::new(),
      meshes: Arena::new(),
      lights: Arena::new(),
      materials: Arena::new(),
      samplers: Arena::new(),
    }
  }

  pub fn update(&mut self) {
    let root = self.get_root_handle();
    self
      .nodes
      .traverse_mut(root, &mut Vec::new(), |this, parent| {
        let node_data = this.data_mut();
        node_data.update(parent.map(|p| p.data()));
        NextTraverseVisit::VisitChildren
      });
  }

  // pub fn create_model(&mut self, creator: impl SceneModelCreator) -> ModelHandle {
  //   creator.create_model(self)
  // }

  // pub fn create_light(&mut self, creator: impl SceneLightCreator) -> LightHandle {
  //   creator.create_light(self)
  // }

  pub fn create_node(&mut self, builder: impl Fn(&mut SceneNode, &mut Self)) -> &mut Self {
    let mut node = SceneNode::default();
    builder(&mut node, self);
    let new = self.nodes.create_node(node);
    let root = self.get_root_handle();
    self.nodes.node_add_child_by_id(root, new);
    self
  }

  // pub fn model_node(&mut self, model: impl SceneModelCreator) -> &mut Self {
  //   let model = self.create_model(model);
  //   self.create_node(|node, _| node.payloads.push(SceneNodePayload::Model(model)));
  //   self
  // }

  // pub fn model_node_with_modify(
  //   &mut self,
  //   model: impl SceneModelCreator,
  //   m: impl Fn(&mut SceneNode),
  // ) -> &mut Self {
  //   let model = self.create_model(model);
  //   self.create_node(|node, _| {
  //     node.payloads.push(SceneNodePayload::Model(model));
  //     m(node)
  //   });
  //   self
  // }

  pub fn background(&mut self, background: impl Background) -> &mut Self {
    self.background = Some(Box::new(background));
    self
  }
}

// pub trait SceneModelCreator<T: SceneBackend> {
//   fn create_model(self, scene: &mut Scene) -> ModelHandle;
// }

// impl SceneModelCreator for <T as SceneBackend>::Model
// where
//   T: SceneBackend,
// {
//   fn create_model(self, scene: &mut Scene) -> ModelHandle {
//     scene.models.insert(self)
//   }
// }

// pub trait SceneLightCreator<T: SceneBackend> {
//   fn create_light(self, scene: &mut Scene) -> LightHandle;
// }

// impl SceneLightCreator for <T as SceneBackend>::Light
// where
//   T: SceneBackend,
// {
//   fn create_light(self, scene: &mut Scene) -> LightHandle {
//     scene.lights.insert(self)
//   }
// }
