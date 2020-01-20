pub trait Element {
    fn render();
}  

pub struct QuadLayout{
    width: f32,
    height: f32,
    left_offset: f32,
    topLoffset: f32,
}

pub struct Div{
    calculated_layout: QuadLayout,
    click_listeners: Vec<Box<dyn FnMut(MouseEvent)>>,
}

impl Div{
    pub fn listener<T: FnMut(MouseEvent) + 'static>(&mut self, func: T) {
      self.click_listeners.push(Box::new(func));
    }

    pub fn render(){

    }
}

pub struct Text{
    content: String,
}