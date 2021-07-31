pub mod ui_impl;
pub use ui_impl::*;

pub mod view;
pub use view::*;

use interphaser::*;
use rendiation_algebra::*;
use rendiation_controller::{ControllerWinitAdapter, OrbitController};
use rendiation_renderable_mesh::tessellation::{
  CubeMeshParameter, IndexedMeshTessellator, SphereMeshParameter,
};
use rendiation_texture::TextureSampler;
use rendiation_webgpu::GPU;
use winit::event::{Event, WindowEvent};

use crate::*;

pub struct Viewer {
  counter: Counter,
  viewer: ViewerInner,
}

impl Viewer {
  pub fn new() -> Self {
    Viewer {
      counter: Counter { count: 0 },
      viewer: ViewerInner {
        content: Viewer3dContent::new(),
        size: (100., 100.),
        ctx: None,
      },
    }
  }
}

pub fn create_ui() -> impl UIComponent<Viewer> {
  button(
    Value::by(|viewer: &Counter| viewer.count.to_string()),
    |viewer: &mut Counter| viewer.count += 10,
  )
  .lens(lens!(Viewer, counter))
  // GPUCanvas::default().lens(lens!(Viewer, viewer))
}

impl CanvasPrinter for ViewerInner {
  fn draw_canvas(&mut self, gpu: &GPU, canvas: &wgpu::TextureView) {
    self.content.update_state();
    self
      .ctx
      .get_or_insert_with(|| {
        Viewer3dRenderingCtx::new(gpu, wgpu::TextureFormat::Rgba8UnormSrgb, self.size)
      })
      .render(canvas, gpu, &mut self.content)
  }

  fn event(&mut self, event: &winit::event::Event<()>) {
    self.content.event(event)
  }

  fn render_size(&self) -> (f32, f32) {
    self.size
  }
}

pub struct ViewerInner {
  content: Viewer3dContent,
  size: (f32, f32),
  ctx: Option<Viewer3dRenderingCtx>,
}

pub struct Viewer3dContent {
  scene: Scene,
  controller: ControllerWinitAdapter<OrbitController>,
}

pub struct Viewer3dRenderingCtx {
  forward: StandardForward,
}

impl Viewer3dRenderingCtx {
  pub fn new(gpu: &GPU, prefer_target_fmt: wgpu::TextureFormat, size: (f32, f32)) -> Self {
    let forward = StandardForward::new(gpu, prefer_target_fmt, size);
    Self { forward }
  }
  pub fn resize_view(&mut self, gpu: &GPU, size: (f32, f32)) {
    self.forward.resize(gpu, size)
  }

  pub fn render(&mut self, target: &wgpu::TextureView, gpu: &GPU, scene: &mut Viewer3dContent) {
    gpu.render(
      &mut RenderPassDispatcher {
        scene: &mut scene.scene,
        pass: &mut self.forward,
      },
      target,
    );
  }
}

impl Viewer3dContent {
  pub fn new() -> Self {
    let mut scene = Scene::new();

    let sampler = scene.add_sampler(TextureSampler::default());

    use image::io::Reader as ImageReader;
    let img = ImageReader::open("/Users/mikialex/Desktop/test.png")
      // let img = ImageReader::open("C:/Users/mk/Desktop/test.png")
      .unwrap()
      .decode()
      .unwrap();
    let img = match img {
      image::DynamicImage::ImageRgba8(img) => img,
      _ => unreachable!(),
    };
    let texture = scene.add_texture2d(img);

    {
      let mesh = SphereMeshParameter::default().tessellate();
      let mesh = scene.add_mesh(mesh);
      let material = BasicMaterial {
        color: Vec3::splat(1.),
        sampler,
        texture,
        states: Default::default(),
      };
      let material = scene.add_material(material);

      let model = MeshModel {
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
      let mut material = BasicMaterial {
        color: Vec3::splat(1.),
        sampler,
        texture,
        states: Default::default(),
      };
      material.states.depth_compare = wgpu::CompareFunction::Always;
      let material = scene.add_material(material);

      let model = MeshModel {
        material,
        mesh,
        group: MeshDrawGroup::Full,
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

    let app = Self { scene, controller };
    app
  }

  pub fn resize_view(&mut self, size: (f32, f32)) {
    if let Some(camera) = &mut self.scene.active_camera {
      let node = self.scene.nodes.get_node_mut(camera.node).data_mut();
      camera.projection.resize(size)
    }
  }

  pub fn event(&mut self, event: &Event<()>) {
    self.controller.event(event);

    if let Event::WindowEvent { event, .. } = event {
      if let WindowEvent::Resized(size) = event {
        self.resize_view((size.width as f32, size.height as f32));
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
