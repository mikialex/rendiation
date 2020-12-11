use rendiation_render_entity::color::{Color, LinearRGBColorSpace, RGBColor};

extern crate image;

pub struct Frame {
  pub data: Vec<Vec<Color<LinearRGBColorSpace<f32>>>>,
}

impl Frame {
  pub fn new(width: usize, height: usize) -> Frame {
    assert!(width >= 1);
    assert!(height >= 1);
    Frame {
      data: vec![vec![Color::from_value((0.0, 0.0, 0.0)); height as usize]; width as usize],
    }
  }

  pub fn width(&self) -> usize {
    self.data.len()
  }
  pub fn height(&self) -> usize {
    self.data[0].len()
  }

  #[allow(clippy::needless_range_loop)]
  pub fn clear(&mut self, color: &Color) {
    let data = &mut self.data;
    for i in 0..data.len() {
      let row = &mut data[i];
      for j in 0..row.len() {
        *data[i][j].mut_r() = color.r();
        *data[i][j].mut_g() = color.g();
        *data[i][j].mut_b() = color.b();
      }
    }
  }

  pub fn set_pixel(&mut self, color: &Color<LinearRGBColorSpace<f32>>, x: u64, y: u64) {
    let data = &mut self.data;
    data[x as usize][y as usize] = *color;
  }

  pub fn pixel_count(&self) -> usize {
    self.width() * self.height()
  }

  pub fn write_to_file(&self, path: &str) {
    let mut img_buf = image::ImageBuffer::new(self.width() as u32, self.height() as u32);

    // Iterate over the coordinates and pixels of the image
    for (x, y, pixel) in img_buf.enumerate_pixels_mut() {
      let pix = self.data[x as usize][y as usize];
      *pixel = image::Rgb([
        (pix.r().min(1.0).max(0.0) * 255.0) as u8,
        (pix.g().min(1.0).max(0.0) * 255.0) as u8,
        (pix.b().min(1.0).max(0.0) * 255.0) as u8,
      ])
    }

    img_buf.save(path).unwrap();
    println!("{} pixels has write to {}", self.pixel_count(), path);
  }
}
