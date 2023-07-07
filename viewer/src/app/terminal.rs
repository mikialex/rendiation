use std::{io::Write, path::Path};

use fast_hash_collection::FastHashMap;
use futures::{executor::ThreadPool, Future, Stream, StreamExt};
use interphaser::{winit::event::VirtualKeyCode, *};
use reactive::SignalStreamExt;
use rendiation_scene_core::Scene;
use webgpu::ReadableTextureBuffer;

use crate::Viewer3dRenderingCtx;

pub struct Terminal {
  pub command_history: Vec<String>,
  pub current_command_editing: String,
  pub command_to_execute: Option<String>,
  pub commands: FastHashMap<String, TerminalCommandCb>,
  pub executor: ThreadPool, // todo should passed in
}

impl Default for Terminal {
  fn default() -> Self {
    let executor = ThreadPool::builder().pool_size(1).create().unwrap();

    Self {
      command_history: Default::default(),
      current_command_editing: Default::default(),
      command_to_execute: Default::default(),
      commands: Default::default(),
      executor,
    }
  }
}

pub struct CommandCtx<'a> {
  pub scene: &'a Scene,
  pub rendering: Option<&'a mut Viewer3dRenderingCtx>,
}

type TerminalCommandCb =
  Box<dyn Fn(&mut CommandCtx, &Vec<String>) -> Box<dyn Future<Output = ()> + Send + Unpin>>;

impl Terminal {
  pub fn mark_execute(&mut self) {
    self.command_to_execute = self.current_command_editing.clone().into();
    self.current_command_editing = String::new();
  }

  pub fn register_command<F, FR>(&mut self, name: impl AsRef<str>, f: F) -> &mut Self
  where
    FR: Future<Output = ()> + Send + Unpin + 'static,
    F: Fn(&mut CommandCtx, &Vec<String>) -> FR + 'static,
  {
    self.commands.insert(
      name.as_ref().to_owned(),
      Box::new(move |c, p| Box::new(f(c, p))),
    );
    self
  }

  pub fn check_execute(&mut self, ctx: &mut CommandCtx) {
    if let Some(command) = self.command_to_execute.take() {
      let parameters: Vec<String> = command
        .split_ascii_whitespace()
        .map(|s| s.to_owned())
        .collect();

      if let Some(command_name) = parameters.first() {
        if let Some(exe) = self.commands.get(command_name) {
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
}

pub fn terminal() -> (impl Component, impl Stream<Item = String> + Unpin) {
  let mut edit_text = Text::default()
    .with_layout(TextLayoutConfig::SizedBox {
      line_wrap: LineWrap::Single,
      horizon_align: TextHorizontalAlignment::Left,
      vertical_align: TextVerticalAlignment::Top,
    })
    .editable();

  let mut current_command = String::new();
  let command_to_execute =
    edit_text
      .events
      .unbound_listen()
      .filter_map_sync(move |e: TextEditMessage| match e {
        TextEditMessage::ContentChange(content) => {
          current_command = content;
          None
        }
        TextEditMessage::KeyboardInput(key) => {
          if let VirtualKeyCode::Return = key {
            let command_to_execute = current_command.clone();
            current_command = String::new();
            Some(command_to_execute)
          } else {
            None
          }
        }
      });

  let clicker = ClickHandler::default();
  let click_event = clicker.events.single_listen().map(|_| {});
  edit_text.set_focus(click_event);

  let text_box = Container::sized((UILength::ParentPercent(100.), UILength::Px(50.)))
    .padding(RectBoundaryWidth::equal(5.))
    .nest_over(edit_text)
    .nest_in(clicker);

  (text_box, command_to_execute)
}

pub fn register_default_commands(terminal: &mut Terminal) {
  terminal.register_command("load-gltf", |ctx, _parameters| {
    let scene = ctx.scene.clone();
    Box::pin(async move {
      use rfd::AsyncFileDialog;

      let file_handle = AsyncFileDialog::new()
        .add_filter("gltf", &["gltf", "glb"])
        .pick_file()
        .await;

      if let Some(file_handle) = file_handle {
        rendiation_scene_gltf_loader::load_gltf(file_handle.path(), &scene).unwrap();
      }
    })
  });

  terminal.register_command("screenshot", |ctx, _parameters| {
    let result = ctx
      .rendering
      .as_mut()
      .map(|cx| cx.read_next_render_result());

    // todo use ?
    Box::pin(async {
      if let Some(result) = result{
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

    })
  });
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
