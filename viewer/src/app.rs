use rendiation_algebra::*;
use rendiation_controller::{ControllerWinitAdapter, OrbitController};
use rendiation_renderable_mesh::tessellation::{IndexedMeshTessellator, SphereMeshParameter};
use rendiation_texture::TextureSampler;
use winit::event::*;

use crate::{
  renderer::Renderer,
  scene::{
    BasicMaterial, IndexBuffer, MaterialCell, Model, RenderPassDispatcher, Scene, SceneMesh,
    StandardForward, VertexBuffer,
  },
};

pub struct Application {
  scene: Scene,
  origin: StandardForward,
  controller: ControllerWinitAdapter<OrbitController>,
}

impl Application {
  pub fn new() -> Self {
    let mut scene = Scene::new();

    let sampler = scene.add_sampler(TextureSampler::default());

    use image::io::Reader as ImageReader;
    let img = ImageReader::open("myimage.png").unwrap().decode().unwrap();
    let img = match img {
      image::DynamicImage::ImageRgba8(img) => img,
      _ => unreachable!(),
    };
    let texture = scene.add_texture2d(img);

    let material = BasicMaterial {
      color: Vec3::splat(1.),
      sampler,
      texture,
    };
    let material = MaterialCell::new(material);
    let material = scene.add_material(material);

    let mesh = SphereMeshParameter::default().tessellate().mesh;
    let mesh = SceneMesh::new(
      vec![VertexBuffer::new(mesh.data)],
      IndexBuffer::new(mesh.index).into(),
    );
    let mesh = scene.add_mesh(mesh);

    let model = Model {
      material,
      mesh,
      node: scene.get_root_handle(),
    };

    let controller = OrbitController::default();
    let controller = ControllerWinitAdapter::new(controller);

    Self {
      scene,
      origin: StandardForward,
      controller,
    }
  }

  pub fn render(&mut self, frame: &wgpu::SwapChainFrame, renderer: &mut Renderer) {
    renderer.render(
      &mut RenderPassDispatcher {
        scene: &mut self.scene,
        style: &mut self.origin,
      },
      frame,
    )
  }

  pub fn update(&mut self, event: &Event<()>) {
    self.controller.event(event)
    //
  }
}
