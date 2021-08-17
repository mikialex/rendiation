use crate::*;

pub struct EditableText {
  text: Text,
  cursor: Option<Cursor>,
}

pub struct Cursor {
  position: UIPosition,
  height: f32,
  text_index: usize,
}

impl<T> Component<T> for EditableText {
  fn event(&mut self, model: &mut T, ctx: &mut EventCtx) {
    self.text.event(model, ctx);

    match ctx.event {
      winit::event::Event::WindowEvent { event, .. } => match event {
        winit::event::WindowEvent::KeyboardInput { input, .. } => {
          // todo handle keyborad input
          // modify text, emit change
          ctx.custom_event.push_event(1);
        }
        _ => {}
      },
      _ => {}
    }
  }

  fn update(&mut self, model: &T, ctx: &mut UpdateCtx) {
    self.text.update(model, ctx)
  }
}
