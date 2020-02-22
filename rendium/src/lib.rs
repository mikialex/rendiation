// pub mod element;
// pub mod event;
// pub mod lens;
pub mod renderer;
// pub use lens::*;
pub use renderer::*;
// pub mod t;
// use event::Event;

pub trait Component<T> {
  fn render(&self, renderer: &mut GUIRenderer);
  fn event(&self, state: &mut T);
}

struct UIState{
  value: f32,
  text: String,
}

struct Slider;

impl Component<UIState> for Slider {
    fn render(&self, renderer: &mut GUIRenderer) { unimplemented!() }
    fn event(&self, state: &mut UIState) { unimplemented!() }
}

pub struct GUI<T> {
  root: Box<dyn Component<T>>,
  size: (f32, f32),
  state: T,
}

impl<T> GUI<T> {
  pub fn new<W, F>(ui_builder: F, init_state: T) -> Self
  where
    W: Component<T> + 'static,
    F: Fn() -> W + 'static,
  {
    let root = Box::new(ui_builder());
    Self {
      root,
      size: (100., 100.),
      state: init_state
    }
  }
}

fn test(){
  let state = UIState{
    value: 1.0,
    text: String::from("test")
  };

  fn create_ui() -> impl Component<UIState>{
    Slider
  }

  let gui = GUI::new(create_ui, state);
  
}