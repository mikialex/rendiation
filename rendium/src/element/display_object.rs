pub struct DisplayList{
    
}

pub enum DisplayObject {
  Quad(Quad),
}

pub struct Quad {
  pub quad: QuadLayout,
  pub color: Vec4<f32>,
}
