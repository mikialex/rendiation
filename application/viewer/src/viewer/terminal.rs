use std::{io::Write, path::Path};

use egui::TextBuffer;
use fast_hash_collection::FastHashMap;
use futures::{executor::ThreadPool, Future};
use rendiation_webgpu::ReadableTextureBuffer;

use crate::*;

pub struct Terminal {
  pub current_input: String,
  pub command_history: Vec<String>,
  pub command_registry: FastHashMap<String, TerminalCommandCb>,
  pub executor: ThreadPool,
}

impl Default for Terminal {
  fn default() -> Self {
    Self {
      current_input: Default::default(),
      command_history: Default::default(),
      command_registry: Default::default(),
      executor: futures::executor::ThreadPool::builder()
        .name_prefix("viewer_io_threads")
        .pool_size(1)
        .create()
        .unwrap(),
    }
  }
}

type TerminalCommandCb =
  Box<dyn Fn(&mut DynCx, &Vec<String>) -> Box<dyn Future<Output = ()> + Send + Unpin>>;

impl Terminal {
  pub fn egui(&mut self, ui: &mut egui::Ui, cx: &mut DynCx) {
    ui.label("terminal");
    let re = ui.text_edit_singleline(&mut self.current_input);
    if re.lost_focus() && re.ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
      self.execute_current(cx);
    }
    ui.end_row();
  }

  pub fn register_command<F, FR>(&mut self, name: impl AsRef<str>, f: F) -> &mut Self
  where
    FR: Future<Output = ()> + Send + 'static,
    F: Fn(&mut DynCx, &Vec<String>) -> FR + 'static,
  {
    self.command_registry.insert(
      name.as_ref().to_owned(),
      Box::new(move |c, p| Box::new(Box::pin(f(c, p)))),
    );
    self
  }

  pub fn register_sync_command<F>(&mut self, name: impl AsRef<str>, f: F) -> &mut Self
  where
    F: Fn(&mut DynCx, &Vec<String>) + 'static + Send + Sync,
  {
    self.register_command(name, move |c, p| {
      f(c, p);
      async {}
    });
    self
  }

  pub fn execute_current(&mut self, ctx: &mut DynCx) {
    let command = self.current_input.take();
    let parameters: Vec<String> = command
      .split_ascii_whitespace()
      .map(|s| s.to_owned())
      .collect();

    if let Some(command_name) = parameters.first() {
      if let Some(exe) = self.command_registry.get(command_name) {
        println!("execute: {command}");

        let task = exe(ctx, &parameters);
        self.executor.spawn_ok(task);
      } else {
        println!("unknown command {command_name}")
      }
      self.command_history.push(command);
    }
  }
}

pub fn register_default_commands(terminal: &mut Terminal) {
  // this mainly to do test
  terminal.register_sync_command("clear-gpu-resource-cache", |ctx, _parameters| {
    access_cx!(ctx, gpu, GPU);

    println!(
      "current gpu resource cache details: {:?}",
      gpu.create_cache_report()
    );
    gpu.clear_resource_cache();
  });

  terminal.register_command("load-gltf", |ctx, _parameters| {
    access_cx!(ctx, scene_cx, Viewer3dSceneCtx);
    let load_target_node = scene_cx.root;
    let load_target_scene = scene_cx.scene;

    async move {
      use rfd::AsyncFileDialog;

      let file_handle = AsyncFileDialog::new()
        .add_filter("gltf", &["gltf", "glb"])
        .pick_file()
        .await;

      let mut writer = SceneWriter::from_global(load_target_scene);

      if let Some(file_handle) = file_handle {
        rendiation_scene_gltf_loader::load_gltf(file_handle.path(), load_target_node, &mut writer)
          .unwrap();
      }
    }
  });

  terminal.register_command("load-obj", |ctx, _parameters| {
    access_cx!(ctx, scene_cx, Viewer3dSceneCtx);
    let load_target_node = scene_cx.root;
    let load_target_scene = scene_cx.scene;

    async move {
      use rfd::AsyncFileDialog;

      let file_handle = AsyncFileDialog::new()
        .add_filter("gltf", &["obj"])
        .pick_file()
        .await;

      let mut writer = SceneWriter::from_global(load_target_scene);

      if let Some(file_handle) = file_handle {
        rendiation_scene_obj_loader::load_obj(
          file_handle.path(),
          load_target_node,
          todo!(),
          &mut writer,
        )
        .unwrap();
      }
    }
  });

  terminal.register_command("export-gltf", |ctx, _parameters| {
    access_cx!(ctx, scene_cx, Viewer3dSceneCtx);
    let export_root_node = scene_cx.root;
    let export_scene = scene_cx.scene;

    async move {
      if let Some(mut dir) = dirs::download_dir() {
        dir.push("gltf_export");

        let reader = rendiation_scene_gltf_exporter::SceneReader::new_from_global(export_scene);

        rendiation_scene_gltf_exporter::build_scene_to_gltf(
          reader,
          export_root_node,
          &dir,
          "scene",
        )
        .unwrap();
      } else {
        log::error!(
          "failed to locate the system's default download directory to write file
  output"
        )
      }
    }
  });

  terminal.register_command("screenshot", |ctx, _parameters| {
    access_cx!(ctx, r, Viewer3dRenderingCtx);
    let result = r.read_next_render_result();

    async {
      match result.await {
          Ok(r) =>{
            if let Some(mut dir) = dirs::download_dir() {
              dir.push("screenshot.png"); // will override old but ok
              write_png(&r, dir);
            }else {
              log::error!("failed to locate the system's default download directory to write viewer screenshot image")
            }
          },
          Err(e) => log::error!("{e:?}"),
      }
    }
  });

  terminal.register_sync_command("fit-camera-view", |ctx, _parameters| {
    access_cx!(ctx, scene_cx, Viewer3dSceneCtx);
    access_cx!(ctx, derived, Viewer3dSceneDerive);
    if let Some(selected) = &scene_cx.selected_target {
      let camera_world = derived.world_mat.access(&scene_cx.camera_node).unwrap();
      // let selected_aabb = derived.world_mat.access(selected).unwrap();
      // let camera_world = fit_camera_view(proj, camera_world, target_world_aabb);
      todo!();
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

  #[cfg(feature = "heap-debug")]
  {
    use crate::GLOBAL_ALLOCATOR;
    terminal.register_sync_command("log-heap-stat", |_ctx, _parameters| {
      let stat = GLOBAL_ALLOCATOR.report();
      println!("{:#?}", stat);
    });
    terminal.register_sync_command("reset-heap-peak", |_ctx, _parameters| {
      GLOBAL_ALLOCATOR.reset_history_peak();
      println!("allocator history peak stat has been reset!");
    });

    terminal.register_sync_command("log-all-type-count-stat", |_ctx, _parameters| {
      let global = heap_tools::HEAP_TOOL_GLOBAL_INSTANCE_COUNTER
        .read()
        .unwrap();
      for (ty, report) in global.report_all_instance_count() {
        println!(
          "{ty} => current: {}, peak: {}",
          report.current, report.history_peak
        );
      }
    });

    terminal.register_sync_command("reset-all-type-count-peak-stat", |_ctx, _parameters| {
      heap_tools::HEAP_TOOL_GLOBAL_INSTANCE_COUNTER
        .write()
        .unwrap()
        .reset_all_instance_history_peak();
      println!("all type instance counter peak stat has been reset!");
    });
  }
}

fn write_png(result: &ReadableTextureBuffer, png_output_path: impl AsRef<Path>) {
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

  let padded_buffer = result.read_raw();
  // from the padded_buffer we write just the unpadded bytes into the image
  for chunk in padded_buffer.chunks(info.padded_bytes_per_row) {
    png_writer
      .write_all(&chunk[..info.unpadded_bytes_per_row])
      .unwrap();
  }
  png_writer.finish().unwrap();
}
