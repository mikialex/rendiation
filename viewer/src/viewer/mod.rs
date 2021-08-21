pub mod examples;
pub use examples::*;

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
  todo: Todo,
  viewer: ViewerInner,
}

impl Viewer {
  pub fn new() -> Self {
    let todo = Todo {
      items: vec![
        TodoItem {
          name: String::from("t1中文测试"),
        },
        TodoItem {
          name: String::from("test 2"),
        },
        TodoItem {
          name: String::from("test 3"),
        },
      ],
    };
    Viewer {
      counter: Counter { count: 0 },
      todo,
      viewer: ViewerInner {
        content: Viewer3dContent::new(),
        size: (0, 0),
        ctx: None,
      },
    }
  }
}

pub fn perf_panel() -> impl UIComponent<Viewer> {
  Text::default()
    .with_line_wrap(LineWrap::Multiple)
    .with_horizon_align(HorizontalAlign::Left)
    .with_vertical_align(VerticalAlign::Top)
    .bind_with_ctx(|s, t, ctx| {
      let content = format!(
        "frame_id: {}\nupdate_time: {}\nlayout_time: {}\nrendering_prepare_time: {}\nrendering_dispatch_time: {}",
        ctx.last_frame_perf_info.frame_id,
        ctx.last_frame_perf_info.update_time.as_micros() as f32 / 1000.,
        ctx.last_frame_perf_info.layout_time.as_micros() as f32 / 1000.,
        ctx.last_frame_perf_info.rendering_prepare_time.as_micros() as f32 / 1000.,
        ctx.last_frame_perf_info.rendering_dispatch_time.as_micros() as f32 / 1000.,
      );
      s.content.set(content);
    })
    .extend(Container::size((500., 200.)))
}

pub fn create_ui() -> impl UIComponent<Viewer> {
  absolute_group()
    .child(AbsChild::new(
      GPUCanvas::default().lens(lens!(Viewer, viewer)),
    ))
    .child(AbsChild::new(build_todo().lens(lens!(Viewer, todo))))
    .child(AbsChild::new(perf_panel()))
    .extend(AbsoluteAnchor::default())
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

  fn update_render_size(&mut self, layout_size: (f32, f32), gpu: &GPU) -> (u32, u32) {
    let new_size = (layout_size.0 as u32, layout_size.1 as u32);
    if let Some(ctx) = &mut self.ctx {
      if self.size != new_size {
        ctx.resize_view(gpu, new_size)
      }
    }
    self.size = new_size;
    new_size
  }
}

pub struct ViewerInner {
  content: Viewer3dContent,
  size: (u32, u32),
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
  pub fn new(gpu: &GPU, prefer_target_fmt: wgpu::TextureFormat, size: (u32, u32)) -> Self {
    let forward = StandardForward::new(gpu, prefer_target_fmt, size);
    Self { forward }
  }
  pub fn resize_view(&mut self, gpu: &GPU, size: (u32, u32)) {
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
    let path = if cfg!(windows) {
      "C:/Users/mk/Desktop/test.png"
    } else {
      "/Users/mikialex/Desktop/test.png"
    };
    let img = ImageReader::open(path).unwrap().decode().unwrap();
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
