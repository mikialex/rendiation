use crate::event::Event;
use crate::renderer::GUIRenderer;
use crate::element::ElementState;
use rendiation_math::Vec2;
use core::any::Any;

struct Message<'a> {
  target: &'a mut dyn Any,
}

struct EventHub {
  listeners: Vec<Box<dyn FnMut(&mut Message)>>,
}

impl EventHub {
  pub fn new() -> Self {
    EventHub {
      listeners: Vec::new(),
    }
  }

  pub fn add<T: FnMut(&mut Message) + 'static>(&mut self, listener: T) {
    self.listeners.push(Box::new(listener));
  }
}

trait Element {
  fn render(&self, renderer: &mut GUIRenderer);
  fn event(&self, message: &mut Message);
  fn get_element_state(&self) -> &ElementState;
  fn is_point_in(&self, point: Vec2<f32>) -> bool;
}

struct ElementFragment {
  elements: Vec<Box<dyn Element>>,
  elements_event: Vec<Vec<usize>>,
  events: EventHub,
}

impl ElementFragment {
  pub fn new() -> Self {
    ElementFragment {
      elements: Vec::new(),
      elements_event:Vec::new(),
      events: EventHub::new(),
    }
  }

  pub fn add_element<T: Element>(&mut self, element: T) -> usize{
    todo!();
  }

  pub fn add_event_listener<T: FnMut(&mut Message) + 'static>(&mut self, element_index:usize, listener: T) -> usize {
    todo!();
  }

  pub fn event<T: Any>(&mut self, payload: &mut T, event: Event, point: Vec2<f32>) {
    // todo!();
    let payload_any = payload as &mut dyn Any;
    let mut message =  Message {
      target: payload_any,
    };
    for element in &mut self.elements {
      if element.is_point_in(point) {
        element.event(&mut message)
      }
    }
  }
}

fn test2(){
  let document = ElementFragment::new();
}

//   fn test(){
//     let mut hub = EventHub {
//       listeners: Vec::new()
//     };
//     hub.add(|m: &mut Message|{
//       let value = m.target.downcast_mut::<bool>().unwrap();
//       println!("{}", value)
//     });
//     hub.add(|m: &mut Message|{
//       let value = m.target.downcast_mut::<usize>().unwrap();
//       println!("{}", value)
//     });
//     let mut test1 = false;
//     let mut m1 = Message {target: &mut test1};
//     let lis1 = &mut hub.listeners[0];
//     lis1(&mut m1);
//     let mut test2 = 0 as usize;
//     let mut m2 = Message {target: &mut test2};
//     let lis2 = &mut hub.listeners[1];
//     lis2(&mut m2);
//   }



trait Component<T> {
  fn render(&self, renderer: &mut GUIRenderer);
  fn event(&self, state: &mut T);
}

struct SlideBar {}

struct UIState{
  value: f32,
  text: String,
}

fn create_ui() -> impl Component<UIState>{
  todo!()
}

