use rendiation_algebra::*;
use rendiation_color::{LinearRGBColor, SRGBColor};

pub struct Frame {
  pub data: Vec<Vec<LinearRGBColor<f32>>>,
}

impl Frame {
  pub fn new(width: usize, height: usize) -> Frame {
    assert!(width >= 1);
    assert!(height >= 1);
    Frame {
      data: vec![vec![Vec3::splat(0.0).into(); height as usize]; width as usize],
    }
  }

  pub fn size(&self) -> Vec2<usize> {
    (self.width(), self.height()).into()
  }

  pub fn width(&self) -> usize {
    self.data.len()
  }
  pub fn height(&self) -> usize {
    self.data[0].len()
  }

  #[allow(clippy::needless_range_loop)]
  pub fn clear(&mut self, color: &LinearRGBColor<f32>) {
    let data = &mut self.data;
    for i in 0..data.len() {
      let row = &mut data[i];
      for j in 0..row.len() {
        data[i][j] = *color;
      }
    }
  }

  pub fn set_pixel(&mut self, color: &LinearRGBColor<f32>, x: u64, y: u64) {
    let data = &mut self.data;
    data[x as usize][y as usize] = *color;
  }

  pub fn pixel_count(&self) -> usize {
    self.width() * self.height()
  }

  pub fn write_to_file(&self, path: &str) {
    println!("writing file to path: {}", path);
    let mut img_buf = image::ImageBuffer::new(self.width() as u32, self.height() as u32);

    // Iterate over the coordinates and pixels of the image
    for (x, y, pixel) in img_buf.enumerate_pixels_mut() {
      let pix = self.data[x as usize][y as usize];
      let pix: SRGBColor<f32> = pix.into();
      *pixel = image::Rgb([
        (pix.r.min(1.0).max(0.0) * 255.0) as u8,
        (pix.g.min(1.0).max(0.0) * 255.0) as u8,
        (pix.b.min(1.0).max(0.0) * 255.0) as u8,
      ])
    }

    img_buf.save(path).unwrap();
    println!("{} pixels has write to {}", self.pixel_count(), path);
  }

  pub fn write_result(&self, name: &str) {
    let mut current_path = std::env::current_dir().unwrap();
    println!("working dir {}", current_path.display());
    current_path.push(String::from(name) + ".png");
    let file_target_path = current_path.into_os_string().into_string().unwrap();
    self.write_to_file(&file_target_path);
  }
}
