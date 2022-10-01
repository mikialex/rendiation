use std::collections::HashMap;

use interphaser::{winit::event::VirtualKeyCode, *};

use crate::{menu, MenuList, MenuModel, UIExamples, Viewer3dContent, ViewerImpl};

pub struct ViewerApplication {
  pub ui_examples: UIExamples,
  pub menu: MenuModel,
  pub viewer: ViewerImpl,
}

impl Default for ViewerApplication {
  fn default() -> Self {
    ViewerApplication {
      ui_examples: Default::default(),
      viewer: Default::default(),
      menu: create_menu(),
    }
  }
}

pub fn create_app() -> impl UIComponent<ViewerApplication> {
  Flex::column().wrap(
    flex_group()
      .child(Child::fixed(menu().lens(lens!(ViewerApplication, menu))))
      .child(Child::flex(
        viewer().lens(lens!(ViewerApplication, viewer)),
        1.,
      )),
  )
}

pub fn viewer() -> impl UIComponent<ViewerImpl> {
  AbsoluteAnchor::default().wrap(
    absolute_group()
      .child(AbsChild::new(GPUCanvas::default()))
      .child(AbsChild::new(terminal().lens(lens!(ViewerImpl, terminal))).with_position((0., 0.)))
      .child(AbsChild::new(perf_panel()).with_position((0., 50.))),
  )
}

fn create_menu() -> MenuModel {
  MenuModel {
    lists: vec![
      MenuList {
        name: "3D Examples".to_string(),
        items: Vec::new(),
      },
      MenuList {
        name: "UI Examples".to_string(),
        items: Vec::new(),
      },
    ],
  }
}

fn perf_panel<T: 'static>() -> impl UIComponent<T> {
  Container::sized((500., 200.))
    .padding(QuadBoundaryWidth::equal(5.))
    .wrap(
    Text::default()
    .with_layout(TextLayoutConfig::SizedBox{
        line_wrap: LineWrap::Multiple,
        horizon_align: TextHorizontalAlignment::Left,
        vertical_align: TextVerticalAlignment::Top,
    })
    .bind_with_ctx(|s, _t, ctx| {
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
  )
}

#[derive(Default)]
pub struct Terminal {
  pub command_history: Vec<String>,
  pub current_command_editing: String,
  pub command_to_execute: Option<String>,
  pub executor: HashMap<String, Box<dyn Fn(&mut Viewer3dContent, &mut Vec<String>)>>,
}

impl Terminal {
  pub fn mark_execute(&mut self) {
    self.command_to_execute = self.current_command_editing.clone().into();
    self.current_command_editing = String::new();
  }

  pub fn register_command(
    &mut self,
    name: impl AsRef<str>,
    f: impl Fn(&mut Viewer3dContent, &mut Vec<String>) + 'static,
  ) -> &mut Self {
    self.executor.insert(name.as_ref().to_owned(), Box::new(f));
    self
  }

  pub fn check_execute(&mut self, content: &mut Viewer3dContent) {
    if let Some(command) = self.command_to_execute.take() {
      let mut parameters: Vec<String> = command
        .split_ascii_whitespace()
        .map(|s| s.to_owned())
        .collect();

      if let Some(first) = parameters.first() {
        if let Some(exe) = self.executor.get(first) {
          println!("execute: {}", command);
          exe(content, &mut parameters)
        } else {
          println!("unknown command {}", first)
        }
        self.command_history.push(command);
      }
    }
  }
}

fn terminal() -> impl UIComponent<Terminal> {
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
