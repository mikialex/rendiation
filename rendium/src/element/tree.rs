use rendiation_math::Vec2;
use super::Element;
use crate::{renderer::GUIRenderer, event::Event};

pub struct ElementsTree<T> {
  elements: Vec<Box<dyn Element<T>>>,
  focus_element_index: Option<usize>
}

impl<T> ElementsTree<T> {
  pub fn event(&mut self, event: &Event, emit_position: Option<Vec2<f32>>) {
    if let Some(position) = emit_position {
      let mut hit_list = Vec::new();
      for element in &self.elements{
        if element.is_point_in(position) {
          hit_list.push(element)
        }
      }
      
    }
  }

  pub fn render(&self, renderer: &mut GUIRenderer) {
    for element in &self.elements{
      element.render(renderer)
    }
  }

  pub fn new() -> Self{
    Self {
      elements: Vec::new(),
      focus_element_index: None,
    }
  }
}
   