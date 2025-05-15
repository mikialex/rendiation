use std::{io::Write, path::Path};

use fast_hash_collection::FastHashMap;
use futures::{executor::ThreadPool, Future};
use rendiation_webgpu::ReadableTextureBuffer;

use crate::*;

pub struct Terminal {
  console: Console,
  pub command_registry: FastHashMap<String, TerminalCommandCb>,
  pub executor: ThreadPool,
  /// some task may only run on main thread
  pub main_thread_tasks: futures::channel::mpsc::UnboundedReceiver<Box<dyn FnOnce() + Send + Sync>>,
  pub ctx: TerminalCtx,
}

#[derive(Clone)]
pub struct TerminalCtx {
  channel: futures::channel::mpsc::UnboundedSender<Box<dyn FnOnce() + Send + Sync>>,
}

impl TerminalCtx {
  pub fn spawn_main_thread<R: 'static + Send + Sync>(
    &self,
    task: impl FnOnce() -> R + Send + Sync + 'static,
  ) -> impl Future<Output = Option<R>> {
    let (s, r) = futures::channel::oneshot::channel();
    self
      .channel
      .unbounded_send(Box::new(|| {
        let result = task();
        s.send(result).ok();
      }))
      .ok();
    r.map(|v| v.ok())
  }
}

impl Default for Terminal {
  fn default() -> Self {
    let (s, r) = futures::channel::mpsc::unbounded();
    let ctx = TerminalCtx { channel: s };

    Self {
      console: Console::new(),
      command_registry: Default::default(),
      executor: futures::executor::ThreadPool::builder()
        .name_prefix("viewer_terminal_task_thread")
        .pool_size(1)
        .create()
        .unwrap(),
      main_thread_tasks: r,
      ctx,
    }
  }
}

type TerminalCommandCb = Box<
  dyn Fn(&mut TerminalInitExecuteCx, &Vec<String>) -> Box<dyn Future<Output = ()> + Send + Unpin>,
>;

pub struct TerminalInitExecuteCx<'a> {
  pub derive: &'a Viewer3dSceneDeriveSource,
  pub scene: &'a Viewer3dSceneCtx,
  pub renderer: &'a mut Viewer3dRenderingCtx,
}

impl Terminal {
  pub fn egui(&mut self, ui: &mut egui::Ui, cx: &mut TerminalInitExecuteCx) {
    let console_response = self.console.ui(ui);
    if let Some(command) = console_response {
      self.execute_current(command, cx);
    }

    noop_ctx!(ctx);
    self
      .main_thread_tasks
      .poll_until_pending(ctx, |task| task());
  }

  pub fn register_command<F, FR>(&mut self, name: impl AsRef<str>, f: F) -> &mut Self
  where
    FR: Future<Output = ()> + Send + 'static,
    F: Fn(&mut TerminalInitExecuteCx, &Vec<String>, &TerminalCtx) -> FR + 'static,
  {
    let cx = self.ctx.clone();
    self.command_registry.insert(
      name.as_ref().to_owned(),
      Box::new(move |c, p| Box::new(Box::pin(f(c, p, &cx)))),
    );
    self
  }

  pub fn register_sync_command<F>(&mut self, name: impl AsRef<str>, f: F) -> &mut Self
  where
    F: Fn(&mut TerminalInitExecuteCx, &Vec<String>) + 'static + Send + Sync,
  {
    self.register_command(name, move |c, p, _| {
      f(c, p);
      async {}
    });
    self
  }

  pub fn execute_current(&mut self, command: String, ctx: &mut TerminalInitExecuteCx) {
    let parameters: Vec<String> = command
      .split_ascii_whitespace()
      .map(|s| s.to_owned())
      .collect();

    if let Some(command_name) = parameters.first() {
      if let Some(exe) = self.command_registry.get(command_name) {
        let task = exe(ctx, &parameters);
        self.executor.spawn_ok(task);
      } else {
        self
          .console
          .writeln(format!("unknown command {command_name}"));
      }
    }
  }
}

pub fn register_default_commands(terminal: &mut Terminal) {
  // this mainly to do test
  terminal.register_sync_command("clear-gpu-resource-cache", |ctx, _parameters| {
    let gpu = ctx.renderer.gpu();
    println!(
      "current gpu resource cache details: {:?}",
      gpu.create_cache_report()
    );
    gpu.clear_resource_cache();
  });

  terminal.register_command("load-gltf", |ctx, _parameters, tcx| {
    let load_target_node = ctx.scene.root;
    let load_target_scene = ctx.scene.scene;
    let tcx = tcx.clone();

    async move {
      use rfd::AsyncFileDialog;

      let file_handle = AsyncFileDialog::new()
        .add_filter("gltf", &["gltf", "glb"])
        .pick_file()
        .await;

      if let Some(file_handle) = file_handle {
        tcx
          .spawn_main_thread(move || {
            let mut writer = SceneWriter::from_global(load_target_scene);

            let load_result = rendiation_scene_gltf_loader::load_gltf(
              file_handle.path(),
              load_target_node,
              &mut writer,
            )
            .unwrap();
            if !load_result.used_but_not_supported_extensions.is_empty() {
              println!(
                "warning: gltf load finished but some used(but not required) extensions are not supported: {:#?}",
                &load_result.used_but_not_supported_extensions
              );
            }
          })
          .await;
      }
    }
  });

  terminal.register_command("load-obj", |ctx, _parameters, tcx| {
    let load_target_node = ctx.scene.root;
    let load_target_scene = ctx.scene.scene;
    let tcx = tcx.clone();

    async move {
      use rfd::AsyncFileDialog;

      let file_handle = AsyncFileDialog::new()
        .add_filter("gltf", &["obj"])
        .pick_file()
        .await;

      if let Some(file_handle) = file_handle {
        tcx
          .spawn_main_thread(move || {
            let mut writer = SceneWriter::from_global(load_target_scene);
            let default_mat = writer.pbr_sg_mat_writer.new_entity();

            rendiation_scene_obj_loader::load_obj(
              file_handle.path(),
              load_target_node,
              default_mat,
              &mut writer,
            )
            .unwrap();
          })
          .await;
      }
    }
  });

  terminal.register_command("export-gltf", |ctx, _parameters, tcx| {
    let derive_update = ctx.derive.poll_update();
    let node_children = derive_update.node_children;
    let mesh_ref_vertex = derive_update.mesh_vertex_ref;
    let sm_ref_s = derive_update.sm_to_s;

    let export_root_node = ctx.scene.root;
    let export_scene = ctx.scene.scene;

    let tcx = tcx.clone();

    async move {
      if let Some(mut dir) = dirs::download_dir() {
        dir.push("gltf_export");

        tcx
          .spawn_main_thread(move || {
            let reader =
              SceneReader::new_from_global(export_scene, mesh_ref_vertex, node_children, sm_ref_s);

            rendiation_scene_gltf_exporter::build_scene_to_gltf(
              reader,
              export_root_node,
              &dir,
              "scene",
            )
            .unwrap();
          })
          .await;
      } else {
        log::error!("failed to locate the system's default download directory to write file output")
      }
    }
  });

  terminal.register_command("screenshot", |ctx, _parameters, _| {
    let result = ctx.renderer.read_next_render_result();

    async {
      match result.await {
          Ok(r) =>{
            if let Some(mut dir) = dirs::download_dir() {
              dir.push("screenshot.png"); // will override old but ok
              write_screenshot(&r, dir);
            }else {
              log::error!("failed to locate the system's default download directory to write viewer screenshot image")
            }
          },
          Err(e) => log::error!("{e:?}"),
      }
    }
  });

  terminal.register_sync_command("fit-camera-view", |ctx, _parameters| {
    let derived = ctx.derive.poll_update();
    if let Some(selected) = &ctx.scene.selected_target {
      let camera_world = derived.world_mat.access(&ctx.scene.camera_node).unwrap();
      let camera_reader = global_entity_component_of::<SceneCameraPerspective>().read();

      let target_world_aabb = derived.sm_world_bounding.access(selected).unwrap();
      let proj = camera_reader.get(ctx.scene.main_camera).unwrap().unwrap();

      let camera_world = fit_camera_view(&proj, camera_world, target_world_aabb);
      // todo fix camera has parent mat
      global_entity_component_of::<SceneNodeLocalMatrixComponent>()
        .write()
        .write(ctx.scene.camera_node, camera_world);
    }
  });

  // terminal.register_command("into-solid-line-mesh", |ctx, _parameters| {
  //   for model in ctx.selection_set.iter_selected() {
  //     let model = model.read();
  //     if let rendiation_scene_core::ModelEnum::Standard(model) = &model.model {
  //       let mesh = model.read().mesh.clone();
  //       let lined_mesh = SolidLinedMesh::new(mesh);
  //       let mesh = MeshEnum::Foreign(Box::new(lined_mesh.into_ptr()));
  //       model.mutate(|mut model| model.modify(StandardModelDelta::mesh(mesh)));
  //     }
  //   }

  //   Box::pin(async move {})
  // });
}

fn write_screenshot(result: &ReadableTextureBuffer, png_output_path: impl AsRef<Path>) {
  let info = result.info();

  let mut png_encoder = png::Encoder::new(
    std::fs::File::create(png_output_path).unwrap(),
    info.width as u32,
    info.height as u32,
  );
  png_encoder.set_depth(png::BitDepth::Eight);
  png_encoder.set_color(png::ColorType::Rgba);

  let mut png_writer = png_encoder
    .write_header()
    .unwrap()
    .into_stream_writer_with_size(info.unpadded_bytes_per_row)
    .unwrap();

  match result.info().format {
    TextureFormat::Rgba8UnormSrgb => {
      let padded_buffer = result.read_raw();
      // from the padded_buffer we write just the unpadded bytes into the image
      for chunk in padded_buffer.chunks(info.padded_bytes_per_row) {
        png_writer
          .write_all(&chunk[..info.unpadded_bytes_per_row])
          .unwrap();
      }
      png_writer.finish().unwrap();
    }
    TextureFormat::Bgra8UnormSrgb => {
      let padded_buffer = result.read_raw();
      for chunk in padded_buffer.chunks(info.padded_bytes_per_row) {
        for [b, g, r, a] in chunk.array_chunks::<4>() {
          png_writer.write_all(&[*r, *g, *b, *a]).unwrap();
        }
      }
      png_writer.finish().unwrap();
    }
    _ => println!("unsupported format: {:?}", result.info().format),
  }
}
