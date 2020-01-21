use crate::event::MouseEvent;

pub trait UINode{
    
}

pub trait Element: UINode {
  fn render();
}

pub struct QuadLayout {
  width: f32,
  height: f32,
  left_offset: f32,
  topLoffset: f32,
}

pub struct Div<C> {
  // calculated_layout: QuadLayout,
  click_listeners: Vec<Box<dyn Fn(&MouseEvent, &mut C)>>,
}

impl<C> Div<C> {
  pub fn new() -> Self {
    Self {
      click_listeners: Vec::new(),
    }
  }

  pub fn listener<T: Fn(&MouseEvent, &mut C) + 'static>(&mut self, func: T) {
    self.click_listeners.push(Box::new(func));
  }

  pub fn trigger_listener(&self, event: &MouseEvent, component_state: &mut C) {
    for listener in self.click_listeners.iter() {
        listener(event, component_state);
      }
  }

  pub fn render() {}
}

pub struct Text {
  content: String,
}
