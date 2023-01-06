use std::collections::HashMap;

use futures::{executor::ThreadPool, Future};
use interphaser::{winit::event::VirtualKeyCode, *};
use rendiation_scene_core::Scene;

use crate::{Viewer3dRenderingCtx, ViewerSnapshotTaskResolver};

pub struct Terminal {
  pub command_history: Vec<String>,
  pub current_command_editing: String,
  pub command_to_execute: Option<String>,
  pub commands: HashMap<String, TerminalCommandCb>,
  pub executor: ThreadPool,
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

pub fn terminal() -> impl UIComponent<Terminal> {
  Container::sized((UILength::ParentPercent(100.), UILength::Px(50.)))
    .padding(QuadBoundaryWidth::equal(5.))
    .wrap(
      Text::default()
        .with_layout(TextLayoutConfig::SizedBox {
          line_wrap: LineWrap::Single,
          horizon_align: TextHorizontalAlignment::Left,
          vertical_align: TextVerticalAlignment::Top,
        })
        .editable()
        .lens(lens!(Terminal, current_command_editing)), //
    )
    .extend(ClickHandler::by(|_, ctx, _| ctx.emit(FocusEditableText)))
    .extend(SimpleHandler::<TextKeyboardInput, _>::by_state(
      simple_handle_in_bubble(),
      |terminal: &mut Terminal, _, e| {
        if let TextKeyboardInput(VirtualKeyCode::Return) = e {
          terminal.mark_execute()
        }
      },
    ))
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
      .map(|cx| ViewerSnapshotTaskResolver::install(cx));

    Box::pin(async {
      if let Some(r) = result {
        let r = r.await.unwrap();
        println!("{}", r.read_raw().len());
      }
    })
  });
}
