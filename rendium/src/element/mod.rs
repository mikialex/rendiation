pub mod quad;
use core::any::Any;
use rendiation_math::*;
pub use quad::*;
use crate::{event::Event, renderer::GUIRenderer};
pub mod tree;

pub trait Element<T> {
  fn render(&self, renderer: &mut GUIRenderer);
  fn event(&self, event: &Event, state: &mut T);
  fn get_element_state(&self) -> &ElementState;
  fn is_point_in(&self, point: Vec2<f32>) -> bool;
}

pub struct ElementState{
  is_active: bool,
  is_hover: bool,
  is_focus: bool,
  z_index: i32,
}

impl ElementState{
  pub fn new() -> Self{
    Self {
      is_active: false,
      is_hover: false,
      is_focus: false,
      z_index: 0,
    }
  }
}

struct Message<'a> {
  target: &'a mut dyn Any
}

struct EventHub {
  listeners: Vec<Box<dyn FnMut(&mut Message)>>
}

impl EventHub {
  fn add<T: FnMut(&mut Message) + 'static>(&mut self, listener: T) {
    self.listeners.push(Box::new(listener));
  }
}


fn test(){
  let mut hub = EventHub {
    listeners: Vec::new()
  };
  hub.add(|m: &mut Message|{
    let value = m.target.downcast_mut::<bool>().unwrap();
    println!("{}", value)
  });
  
  hub.add(|m: &mut Message|{
    let value = m.target.downcast_mut::<usize>().unwrap();
    println!("{}", value)
  });
  let mut test1 = false;
  let mut m1 = Message {target: &mut test1};
  let lis1 = &mut hub.listeners[0];
  lis1(&mut m1);

  let mut test2 = 0 as usize;
  let mut m2 = Message {target: &mut test2};
  let lis2 = &mut hub.listeners[1];
  lis2(&mut m2);
}