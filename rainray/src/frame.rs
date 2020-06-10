use rendiation_render_entity::color::{RGBColor, Color};

extern crate image;

pub struct Frame {
  pub width: u64,
  pub height: u64,
  pub data: Vec<Vec<Color>>,
}

impl Frame {
  pub fn new(width: u64, height: u64) -> Frame {
    Frame {
      width,
      height,
      data: vec![vec![Color::from_value((0.0, 0.0, 0.0)); height as usize]; width as usize],
    }
  }

  pub fn clear(&mut self, color: &Color) {
    let data = &mut self.data;
    for i in 0..data.len() {
      let row = &mut data[i];
      for j in 0..row.len() {
        data[i][j].mut_r(color.r());
        data[i][j].mut_g(color.g());
        data[i][j].mut_b(color.b());
      }
    }
  }

  #[allow(dead_code)]
  pub fn set_pixel(&mut self, color: &Color, x: u64, y: u64) {
    let data = &mut self.data;
    data[x as usize][y as usize] = *color;
  }

  pub fn pixel_count(&self) -> u64 {
    self.width * self.height
  }

  pub fn write_to_file(&self, path: &str) {
    let mut imgbuf = image::ImageBuffer::new(self.width as u32, self.height as u32);

    // Iterate over the coordinates and pixels of the image
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
      let pix = self.data[x as usize][y as usize];
      *pixel = image::Rgb([
        (pix.r.min(1.0).max(0.0) * 255.0) as u8,
        (pix.g.min(1.0).max(0.0) * 255.0) as u8,
        (pix.b.min(1.0).max(0.0) * 255.0) as u8,
      ])
    }

    imgbuf.save(path).unwrap();
    println!("{} pixels has write to {}", self.pixel_count(), path);
  }

  // pub fn iter_pixels(){

  // }
}

// impl Iterator for MyFunkyIterator {
//     type Item = (f32, Position);

//     fn next(&mut self) -> Option<(f32, Position)> {
//         // @target_san's example has the inner iterator at self.0
//         // so maybe call self.0.next(), tweak the result, and return it.
//     }
// }
