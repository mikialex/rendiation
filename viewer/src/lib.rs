pub mod materials;
use arena::Arena;
use arena_tree::NextTraverseVisit;
pub use materials::*;

pub mod buffer;
pub use buffer::*;
pub mod renderer;
pub use renderer::*;

pub struct WebGPUScene;

mod swap_chain;

pub trait Light {}
pub trait Background {}
pub trait Mesh {
  fn setup_pass<'a>(&mut self, renderer: &'a Renderer, pass: &mut wgpu::RenderPass<'a>);
}

pub trait Model {
  fn update(&mut self, ctx: &mut SceneRenderCtx, renderer: &mut Renderer);
  fn render<'a>(
    &self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &mut SceneRenderCtx,
  );
}

pub trait GPUSceneExt {
  //
}

pub trait Renderable {
  fn render<'a>(
    &mut self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    des: &wgpu::RenderPassDescriptor,
  );
}

pub struct SceneRenderCtx<'a> {
  materials: &'a mut Arena<Box<dyn Material>>,
  meshes: &'a mut Arena<Box<dyn Mesh>>,
  material_ctx: SceneMaterialRenderPrepareCtx,
}

impl Renderable for Scene {
  fn render<'a>(
    &mut self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    des: &wgpu::RenderPassDescriptor,
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
      model.render(renderer, pass, &mut ctx)
    })
  }
}

pub struct SceneMesh {
  vertex: Vec<VertexBuffer>,
  index: Option<IndexBuffer>,
}

impl Mesh for SceneMesh {
  fn setup_pass<'a>(&mut self, renderer: &'a Renderer, pass: &mut wgpu::RenderPass<'a>) {
    self
      .index
      .as_mut()
      .map(|index| index.setup_pass(renderer, pass));
    self
      .vertex
      .iter_mut()
      .enumerate()
      .for_each(|(i, vertex)| vertex.setup_pass(renderer, pass, i as u32))
  }
}

pub struct SceneModel {
  material: MaterialHandle,
  mesh: MeshHandle,
}

impl Model for SceneModel {
  fn update(&mut self, ctx: &mut SceneRenderCtx, renderer: &mut Renderer) {
    let material = ctx.materials.get_mut(self.material).unwrap();
    material.update(renderer, &mut ctx.material_ctx)
  }

  fn render<'a>(
    &self,
    renderer: &'a Renderer,
    pass: &mut wgpu::RenderPass<'a>,
    ctx: &mut SceneRenderCtx,
  ) {
    let material = ctx.materials.get_mut(self.material).unwrap();
    material.setup_pass(renderer, pass);
    let mesh = ctx.meshes.get_mut(self.mesh).unwrap();
    mesh.setup_pass(renderer, pass);
  }
}
