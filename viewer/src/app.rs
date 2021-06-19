use rendiation_algebra::*;
use rendiation_controller::{ControllerWinitAdapter, OrbitController};
use rendiation_renderable_mesh::tessellation::{
  CubeMeshParameter, IndexedMeshTessellator, SphereMeshParameter,
};
use rendiation_texture::TextureSampler;
use winit::event::*;

use crate::{
  renderer::Renderer,
  scene::{
    BasicMaterial, Camera, MeshDrawGroup, Model, RenderPassDispatcher, Scene, StandardForward,
  },
};

pub struct Application {
  scene: Scene,
  forward: StandardForward,
  controller: ControllerWinitAdapter<OrbitController>,
}

impl Application {
  pub fn new(renderer: &mut Renderer, size: (f32, f32)) -> Self {
    let mut scene = Scene::new();

    let sampler = scene.add_sampler(TextureSampler::default());

    use image::io::Reader as ImageReader;
    let img = ImageReader::open("C:/Users/mk/Desktop/test.png")
      .unwrap()
      .decode()
      .unwrap();
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
    let material = scene.add_material(material);

    {
      let mesh = SphereMeshParameter::default().tessellate();
      let mesh = scene.add_mesh(mesh);

      let model = Model {
        material,
        mesh,
        group: MeshDrawGroup::Full,
        node: scene.get_root_handle(),
      };

      scene.add_model(model);
    }

    {
      let mesh = CubeMeshParameter::default().tessellate();
      let mesh = scene.add_mesh(mesh);

      let model = Model {
        material,
        mesh,
        group: MeshDrawGroup::SubMesh(1),
        node: scene.get_root_handle(),
      };

      scene.add_model(model);
    }

    let camera = PerspectiveProjection::default();
    let camera_node = scene.create_node(|node, _| {
      node.local_matrix = Mat4::lookat(Vec3::splat(10.), Vec3::splat(0.), Vec3::new(0., 1., 0.));
    });
    let camera = Camera::new(camera, camera_node);
    scene.active_camera = camera.into();

    let controller = OrbitController::default();
    let controller = ControllerWinitAdapter::new(controller);

    let forward = StandardForward::new(&renderer.device, size);

    let mut app = Self {
      scene,
      forward,
      controller,
    };
    app.resize_view(&renderer.device, size);
    app
  }

  pub fn render(&mut self, frame: &wgpu::SwapChainFrame, renderer: &mut Renderer) {
    renderer.render(
      &mut RenderPassDispatcher {
        scene: &mut self.scene,
        style: &mut self.forward,
      },
      frame,
    )
  }

  pub fn resize_view(&mut self, device: &wgpu::Device, size: (f32, f32)) {
    if let Some(camera) = &mut self.scene.active_camera {
      let node = self.scene.nodes.get_node_mut(camera.node).data_mut();
      camera.projection.resize(size)
    }
    self.forward.resize(device, size)
  }

  pub fn event(&mut self, renderer: &mut Renderer, event: &Event<()>) {
    self.controller.event(event);
    if let Event::WindowEvent { event, .. } = event {
      if let WindowEvent::Resized(size) = event {
        self.resize_view(&renderer.device, (size.width as f32, size.height as f32));
      }
    }
  }

  pub fn update_state(&mut self) {
    if let Some(camera) = &mut self.scene.active_camera {
      let node = self.scene.nodes.get_node_mut(camera.node).data_mut();
      self.controller.update(node);
    }
  }
}
