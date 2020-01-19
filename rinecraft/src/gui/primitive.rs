pub trait Primitive {
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
}

impl Div{
    pub fn listener(){

    }

    pub fn render(){

    }
}

pub struct Text{
    content: String,
}